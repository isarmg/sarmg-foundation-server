use super::*;
use sarmg_admin_core::{
    AdministratorManagementContext, AdministratorMutation, AuditOutcome, ManagementError,
};
use std::time::{SystemTime, UNIX_EPOCH};

type Failure = ManagementError<Error>;

fn storage_error(error: sqlx::Error) -> Failure {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        ManagementError::Conflict
    } else {
        ManagementError::Store(Error::Sqlx(error))
    }
}

pub(super) async fn execute(
    store: &SqliteAdministratorStore,
    context: &AdministratorManagementContext,
    mutation: AdministratorMutation,
) -> Result<(), Failure> {
    context
        .validate()
        .map_err(|_| ManagementError::InvalidInput)?;
    // Serialize authorization, last-administrator checks, data and audit writes.
    // Read the clock AFTER waiting for the writer lock and password hashing.
    let mut transaction = store
        .pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(storage_error)?;
    let wall_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ManagementError::Store(Error::IntegerRange))?
        .as_micros();
    let now = u64::try_from(wall_now)
        .map_err(|_| ManagementError::Store(Error::IntegerRange))?
        .max(context.now_micros);
    let timestamp = to_i64(now).map_err(ManagementError::Store)?;
    let authorized: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM _sarmg_admin_sessions s JOIN _sarmg_administrators a ON a.administrator_id=s.administrator_id WHERE s.session_id=? AND s.administrator_id=? AND s.csrf_hash=? AND s.revoked_at_micros IS NULL AND s.idle_expires_at_micros>? AND s.absolute_expires_at_micros>? AND a.active=1 AND a.session_version=s.administrator_session_version)"
    ).bind(context.identity.session_id.as_str()).bind(context.identity.administrator_id.as_str())
        .bind(context.identity.csrf_hash.as_slice()).bind(timestamp).bind(timestamp)
        .fetch_one(&mut *transaction).await.map_err(storage_error)?;
    if !authorized {
        return Err(ManagementError::Unauthorized);
    }

    let (subject, action, revokes_sessions) = match mutation {
        AdministratorMutation::UpdateOwnAccount {
            username,
            expected_password_hash,
            password_hash,
        } => {
            sarmg_admin_auth::require_canonical_administrator_username(&username)
                .map_err(|_| ManagementError::InvalidInput)?;
            if let Some(hash) = &password_hash {
                sarmg_admin_auth::require_current_password_hash(hash)
                    .map_err(|_| ManagementError::InvalidInput)?;
            }
            let id = &context.identity.administrator_id;
            let changed = sqlx::query("UPDATE _sarmg_administrators SET username=?, password_hash=COALESCE(?, password_hash), session_version=session_version+1, updated_at_micros=? WHERE administrator_id=? AND active=1 AND password_hash=?")
                .bind(&username).bind(password_hash).bind(timestamp).bind(id.as_str()).bind(expected_password_hash)
                .execute(&mut *transaction).await.map_err(storage_error)?.rows_affected();
            if changed != 1 {
                return Err(ManagementError::Unauthorized);
            }
            revoke_administrator_sessions(&mut transaction, id, now)
                .await
                .map_err(ManagementError::Store)?;
            (username, SecurityAction::AdministratorAccountUpdated, true)
        }
        AdministratorMutation::Create(_) => return Err(ManagementError::Conflict),
        AdministratorMutation::ChangePassword {
            administrator_id,
            password_hash,
        } => {
            sarmg_admin_auth::require_current_password_hash(&password_hash)
                .map_err(|_| ManagementError::InvalidInput)?;
            let subject = subject(&mut transaction, &administrator_id).await?;
            sqlx::query("UPDATE _sarmg_administrators SET password_hash=?, session_version=session_version+1, updated_at_micros=? WHERE administrator_id=?")
                .bind(password_hash).bind(timestamp).bind(administrator_id.as_str())
                .execute(&mut *transaction).await.map_err(storage_error)?;
            revoke_administrator_sessions(&mut transaction, &administrator_id, now)
                .await
                .map_err(ManagementError::Store)?;
            (subject, SecurityAction::AdministratorPasswordChanged, true)
        }
        AdministratorMutation::Disable { administrator_id } => {
            let subject = subject(&mut transaction, &administrator_id).await?;
            let count: i64 =
                sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_administrators WHERE active=1")
                    .fetch_one(&mut *transaction)
                    .await
                    .map_err(storage_error)?;
            if count <= 1 {
                return Err(ManagementError::LastAdministrator);
            }
            sqlx::query("UPDATE _sarmg_administrators SET active=0, session_version=session_version+1, updated_at_micros=? WHERE administrator_id=?")
                .bind(timestamp).bind(administrator_id.as_str()).execute(&mut *transaction).await.map_err(storage_error)?;
            revoke_administrator_sessions(&mut transaction, &administrator_id, now)
                .await
                .map_err(ManagementError::Store)?;
            (subject, SecurityAction::AdministratorDisabled, true)
        }
    };
    let primary_event = event(context, &subject, action, now)?;
    insert_audit(&mut transaction, &primary_event)
        .await
        .map_err(ManagementError::Store)?;
    if revokes_sessions {
        let event = event(
            context,
            &subject,
            SecurityAction::AdministratorSessionsRevoked,
            now,
        )?;
        insert_audit(&mut transaction, &event)
            .await
            .map_err(ManagementError::Store)?;
    }
    transaction.commit().await.map_err(storage_error)
}

async fn subject(
    transaction: &mut Transaction<'_, Sqlite>,
    id: &Identifier,
) -> Result<String, Failure> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT username, active FROM _sarmg_administrators WHERE administrator_id=?",
    )
    .bind(id.as_str())
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage_error)?;
    match row {
        Some((username, true)) => Ok(username),
        Some((_, false)) => Err(ManagementError::Conflict),
        None => Err(ManagementError::NotFound),
    }
}

fn event(
    context: &AdministratorManagementContext,
    subject: &str,
    action: SecurityAction,
    now: u64,
) -> Result<SecurityAuditEvent, Failure> {
    let token = sarmg_admin_auth::random_token()
        .map_err(|error| ManagementError::Store(Error::Authentication(error)))?;
    Ok(SecurityAuditEvent {
        event_id: Identifier::new(token)
            .map_err(|error| ManagementError::Store(Error::Core(error)))?,
        action,
        outcome: AuditOutcome::Success,
        actor_administrator_id: Some(context.identity.administrator_id.clone()),
        subject_digest: Some(sarmg_admin_auth::token_hash(subject)),
        request_id: context.request_id.clone(),
        detail_json: "{}".into(),
        occurred_at_micros: now,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarmg_admin_core::{AdministratorService, LoginContext};

    struct Fixture {
        _directory: tempfile::TempDir,
        service: AdministratorService<SqliteAdministratorStore>,
        context: AdministratorManagementContext,
    }

    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros()
            .try_into()
            .unwrap()
    }

    async fn fixture() -> Fixture {
        let directory = tempfile::tempdir().unwrap();
        let pool = sarmg_sqlite::create_if_missing(
            directory.path().join("admin.sqlite3"),
            sarmg_sqlite::PoolOptions::new(4),
        )
        .await
        .unwrap();
        sqlx::raw_sql(ADMIN_PERSISTENT_DDL)
            .execute(&pool)
            .await
            .unwrap();
        let service = AdministratorService::new(SqliteAdministratorStore::new(pool));
        service
            .bootstrap_administrator("admin", "correct horse battery", now())
            .await
            .unwrap();
        let login = service
            .login(
                "admin",
                "correct horse battery",
                &LoginContext {
                    source: "127.0.0.1".into(),
                    request_id: None,
                    now_micros: now(),
                },
            )
            .await
            .unwrap();
        let identity = service
            .authenticate_session(&login.session_token, now())
            .await
            .unwrap();
        Fixture {
            _directory: directory,
            service,
            context: AdministratorManagementContext {
                identity,
                request_id: Some("management-test".into()),
                now_micros: now(),
            },
        }
    }

    async fn create(fixture: &Fixture, name: &str) -> AdministratorRecord {
        sqlx::query("INSERT INTO _sarmg_administrators(administrator_id, username, password_hash, active, session_version, created_at_micros, updated_at_micros) SELECT ?, ?, password_hash, 1, 1, created_at_micros+1, updated_at_micros FROM _sarmg_administrators WHERE username='admin'")
            .bind(format!("legacy-{name}")).bind(name).execute(fixture.service.store().pool()).await.unwrap();
        fixture
            .service
            .store()
            .administrator_by_username(name)
            .await
            .unwrap()
            .unwrap()
    }

    #[tokio::test]
    async fn own_account_update_is_atomic_and_keeps_identity() {
        let f = fixture().await;
        let original = f.context.identity.administrator_id.clone();
        f.service
            .update_own_account(
                &f.context,
                "renamed",
                "correct horse battery",
                Some("updated correct password"),
            )
            .await
            .unwrap();
        assert!(
            f.service
                .store()
                .administrator_by_username("admin")
                .await
                .unwrap()
                .is_none()
        );
        let account = f
            .service
            .store()
            .administrator_by_username("renamed")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(account.administrator_id, original);
        assert!(sarmg_admin_auth::verify_password(
            "updated correct password",
            &account.password_hash
        ));
        let active: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_admin_sessions WHERE administrator_id=? AND revoked_at_micros IS NULL")
            .bind(original.as_str()).fetch_one(f.service.store().pool()).await.unwrap();
        assert_eq!(active, 0);
        assert!(matches!(
            f.service
                .update_own_account(&f.context, "again", "updated correct password", None)
                .await,
            Err(ManagementError::Unauthorized)
        ));
        let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_security_audit_events WHERE action='administrator.account_updated'").fetch_one(f.service.store().pool()).await.unwrap();
        assert_eq!(events, 1);
    }

    #[tokio::test]
    async fn wrong_password_and_duplicate_name_leave_account_unchanged() {
        let f = fixture().await;
        create(&f, "secondary").await;
        assert!(matches!(
            f.service
                .update_own_account(&f.context, "renamed", "incorrect password", None)
                .await,
            Err(ManagementError::InvalidCurrentPassword)
        ));
        assert!(matches!(
            f.service
                .update_own_account(
                    &f.context,
                    "secondary",
                    "correct horse battery",
                    Some("updated correct password")
                )
                .await,
            Err(ManagementError::Conflict)
        ));
        let account = f
            .service
            .store()
            .administrator_by_username("admin")
            .await
            .unwrap()
            .unwrap();
        assert!(sarmg_admin_auth::verify_password(
            "correct horse battery",
            &account.password_hash
        ));
        assert_eq!(account.session_version, 1);
        f.service
            .update_own_account(&f.context, "renamed", "correct horse battery", None)
            .await
            .unwrap();
        let account = f
            .service
            .store()
            .administrator_by_username("renamed")
            .await
            .unwrap()
            .unwrap();
        assert!(sarmg_admin_auth::verify_password(
            "correct horse battery",
            &account.password_hash
        ));
    }

    #[tokio::test]
    async fn stale_touch_or_restore_cannot_resurrect_a_rotated_csrf_hash() {
        let f = fixture().await;
        let store = f.service.store();
        let id = &f.context.identity.session_id;
        let old = f.context.identity.csrf_hash;
        let current = [42; 32];
        let timestamp = now();
        assert!(
            store
                .rotate_session_csrf(id, old, current, timestamp, timestamp + 1_000_000)
                .await
                .unwrap()
        );
        assert!(
            !store
                .rotate_session_csrf(id, old, old, timestamp + 1, timestamp + 1_000_000)
                .await
                .unwrap()
        );
        assert!(
            !store
                .rotate_session_csrf(id, old, [43; 32], timestamp + 2, timestamp + 1_000_000)
                .await
                .unwrap()
        );
        assert!(
            store
                .rotate_session_csrf(id, current, current, timestamp - 1, timestamp + 1_000_000)
                .await
                .unwrap()
        );
        let stored: Vec<u8> =
            sqlx::query_scalar("SELECT csrf_hash FROM _sarmg_admin_sessions WHERE session_id=?")
                .bind(id.as_str())
                .fetch_one(store.pool())
                .await
                .unwrap();
        assert_eq!(stored, current);
        assert!(matches!(
            f.service
                .disable_administrator(&f.context, f.context.identity.administrator_id.as_str())
                .await,
            Err(ManagementError::Unauthorized)
        ));
    }

    #[tokio::test]
    async fn only_one_administrator_can_be_created_and_legacy_access_is_revoked() {
        let f = fixture().await;
        assert!(matches!(
            f.service
                .create_administrator(&f.context, "other", "another correct password")
                .await,
            Err(ManagementError::Conflict)
        ));
        assert!(
            !f.service
                .bootstrap_administrator("other", "another correct password", now())
                .await
                .unwrap()
        );
        let legacy = create(&f, "legacy").await;
        f.service
            .store()
            .validate_all_administrators()
            .await
            .unwrap();
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_administrators WHERE active=1")
                .fetch_one(f.service.store().pool())
                .await
                .unwrap();
        assert_eq!(count, 1);
        assert!(
            !f.service
                .store()
                .administrator_by_username(&legacy.username)
                .await
                .unwrap()
                .unwrap()
                .active
        );
        assert!(matches!(
            f.service
                .disable_administrator(&f.context, f.context.identity.administrator_id.as_str())
                .await,
            Err(ManagementError::LastAdministrator)
        ));
    }

    #[tokio::test]
    async fn account_audit_failure_rolls_back_password_identity_and_session_revocation() {
        let f = fixture().await;
        sqlx::raw_sql("CREATE TRIGGER reject_audit BEFORE INSERT ON _sarmg_security_audit_events BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END;")
            .execute(f.service.store().pool()).await.unwrap();
        assert!(matches!(
            f.service
                .update_own_account(
                    &f.context,
                    "changed",
                    "correct horse battery",
                    Some("updated correct password")
                )
                .await,
            Err(ManagementError::Store(_))
        ));
        let original = f
            .service
            .store()
            .administrator_by_username("admin")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(original.session_version, 1);
        assert!(sarmg_admin_auth::verify_password(
            "correct horse battery",
            &original.password_hash
        ));
        let revoked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _sarmg_admin_sessions WHERE revoked_at_micros IS NOT NULL",
        )
        .fetch_one(f.service.store().pool())
        .await
        .unwrap();
        assert_eq!(revoked, 0);
    }

    #[tokio::test]
    async fn minute_activity_updates_can_finish_out_of_order_without_logging_out() {
        let f = fixture().await;
        let store = f.service.store();
        let id = &f.context.identity.session_id;
        let hash = f.context.identity.csrf_hash;
        let timestamp = now() + 61_000_000;
        let deadline = timestamp + sarmg_admin_core::SESSION_IDLE_MICROS;
        assert!(
            store
                .rotate_session_csrf(id, hash, hash, timestamp + 100, deadline + 100)
                .await
                .unwrap()
        );
        assert!(
            store
                .rotate_session_csrf(id, hash, hash, timestamp, deadline)
                .await
                .unwrap()
        );
        let row: (i64, i64) = sqlx::query_as("SELECT last_seen_at_micros, idle_expires_at_micros FROM _sarmg_admin_sessions WHERE session_id=?").bind(id.as_str()).fetch_one(store.pool()).await.unwrap();
        assert_eq!(row, ((timestamp + 100) as i64, (deadline + 100) as i64));
    }
}
