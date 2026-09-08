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
        AdministratorMutation::Create(mut record) => {
            record.created_at_micros = now;
            record.updated_at_micros = now;
            record
                .validate()
                .map_err(|_| ManagementError::InvalidInput)?;
            if !record.active
                || record.session_version != 1
                || record.last_login_at_micros.is_some()
            {
                return Err(ManagementError::InvalidInput);
            }
            sqlx::query("INSERT INTO _sarmg_administrators(administrator_id, username, password_hash, active, session_version, created_at_micros, updated_at_micros, last_login_at_micros) VALUES(?, ?, ?, 1, 1, ?, ?, NULL)")
                .bind(record.administrator_id.as_str()).bind(&record.username).bind(&record.password_hash)
                .bind(timestamp).bind(timestamp).execute(&mut *transaction).await.map_err(storage_error)?;
            (record.username, SecurityAction::AdministratorCreated, false)
        }
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
    use sarmg_admin_core::{AdministratorService, AuthenticatedIdentity, LoginContext};

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
        fixture
            .service
            .create_administrator(&fixture.context, name, "another correct password")
            .await
            .unwrap();
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
            !store
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
    async fn creates_lists_disables_and_protects_the_final_administrator() {
        let f = fixture().await;
        assert!(matches!(
            f.service
                .disable_administrator(&f.context, f.context.identity.administrator_id.as_str())
                .await,
            Err(ManagementError::LastAdministrator)
        ));
        let secondary = create(&f, "secondary").await;
        assert!(matches!(
            f.service
                .create_administrator(&f.context, "SECONDARY", "another correct password")
                .await,
            Err(ManagementError::Conflict)
        ));
        assert_eq!(
            f.service.list_administrators(1, 1).await.unwrap()[0].username,
            "secondary"
        );
        assert!(matches!(
            f.service.list_administrators(101, 0).await,
            Err(ManagementError::InvalidInput)
        ));
        f.service
            .disable_administrator(&f.context, secondary.administrator_id.as_str())
            .await
            .unwrap();
        assert!(
            !f.service
                .store()
                .administrator_by_username("secondary")
                .await
                .unwrap()
                .unwrap()
                .active
        );
        assert!(matches!(
            f.service
                .disable_administrator(&f.context, secondary.administrator_id.as_str())
                .await,
            Err(ManagementError::Conflict)
        ));
        let events: Vec<(String, String, Vec<u8>, String, String)> = sqlx::query_as("SELECT action, actor_administrator_id, subject_digest, request_id, detail_json FROM _sarmg_security_audit_events WHERE request_id='management-test' ORDER BY occurred_at_micros, action")
            .fetch_all(f.service.store().pool()).await.unwrap();
        assert_eq!(events.len(), 3);
        for (_, actor, digest, request_id, detail) in events {
            assert_eq!(actor, f.context.identity.administrator_id.as_str());
            assert_eq!(digest, sarmg_admin_auth::token_hash("secondary"));
            assert_eq!(request_id, "management-test");
            assert_eq!(detail, "{}");
        }
    }

    #[tokio::test]
    async fn password_change_revokes_sessions_and_rejects_stale_authorization() {
        let f = fixture().await;
        f.service
            .set_administrator_password(
                &f.context,
                f.context.identity.administrator_id.as_str(),
                "replacement correct password",
            )
            .await
            .unwrap();
        assert!(matches!(
            f.service
                .create_administrator(&f.context, "secondary", "another correct password")
                .await,
            Err(ManagementError::Unauthorized)
        ));
        let record = f
            .service
            .store()
            .administrator_by_username("admin")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(record.session_version, 2);
        let active: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _sarmg_admin_sessions WHERE revoked_at_micros IS NULL",
        )
        .fetch_one(f.service.store().pool())
        .await
        .unwrap();
        assert_eq!(active, 0);
    }

    #[tokio::test]
    async fn csrf_rotation_and_expiry_are_rechecked_using_the_commit_clock() {
        let f = fixture().await;
        let mut stale = f.context.clone();
        stale.identity = AuthenticatedIdentity {
            csrf_hash: [0; 32],
            ..stale.identity
        };
        assert!(matches!(
            f.service
                .create_administrator(&stale, "secondary", "another correct password")
                .await,
            Err(ManagementError::Unauthorized)
        ));
        // A request timestamp before expiry must not authorize a write after it.
        let expired = now() - 1;
        sqlx::query("UPDATE _sarmg_admin_sessions SET idle_expires_at_micros=?")
            .bind(i64::try_from(expired).unwrap())
            .execute(f.service.store().pool())
            .await
            .unwrap();
        assert!(f.context.now_micros < expired);
        assert!(matches!(
            f.service
                .create_administrator(&f.context, "secondary", "another correct password")
                .await,
            Err(ManagementError::Unauthorized)
        ));
        assert_eq!(f.service.store().administrator_count().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn audit_failure_rolls_back_administrators_passwords_and_revocations() {
        let f = fixture().await;
        let secondary = create(&f, "secondary").await;
        sqlx::raw_sql("CREATE TRIGGER reject_audit BEFORE INSERT ON _sarmg_security_audit_events BEGIN SELECT RAISE(ABORT, 'injected audit failure'); END;")
            .execute(f.service.store().pool()).await.unwrap();
        assert!(matches!(
            f.service
                .create_administrator(&f.context, "third", "another correct password")
                .await,
            Err(ManagementError::Store(_))
        ));
        assert!(matches!(
            f.service
                .disable_administrator(&f.context, secondary.administrator_id.as_str())
                .await,
            Err(ManagementError::Store(_))
        ));
        assert!(matches!(
            f.service
                .set_administrator_password(
                    &f.context,
                    f.context.identity.administrator_id.as_str(),
                    "replacement correct password"
                )
                .await,
            Err(ManagementError::Store(_))
        ));
        assert!(
            f.service
                .store()
                .administrator_by_username("third")
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            f.service
                .store()
                .administrator_by_username("secondary")
                .await
                .unwrap()
                .unwrap()
                .active
        );
        let admin = f
            .service
            .store()
            .administrator_by_username("admin")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(admin.session_version, 1);
        let revoked: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM _sarmg_admin_sessions WHERE revoked_at_micros IS NOT NULL",
        )
        .fetch_one(f.service.store().pool())
        .await
        .unwrap();
        assert_eq!(revoked, 0);
    }

    #[tokio::test]
    async fn concurrent_disable_preserves_one_active_administrator() {
        let f = fixture().await;
        let secondary = create(&f, "secondary").await;
        let login = f
            .service
            .login(
                "secondary",
                "another correct password",
                &LoginContext {
                    source: "127.0.0.1".into(),
                    request_id: None,
                    now_micros: now(),
                },
            )
            .await
            .unwrap();
        let other = AdministratorManagementContext {
            identity: f
                .service
                .authenticate_session(&login.session_token, now())
                .await
                .unwrap(),
            request_id: Some("other-management".into()),
            now_micros: now(),
        };
        let (first, second) = tokio::join!(
            f.service
                .disable_administrator(&f.context, secondary.administrator_id.as_str()),
            f.service
                .disable_administrator(&other, f.context.identity.administrator_id.as_str()),
        );
        assert_ne!(first.is_ok(), second.is_ok());
        assert!(matches!(
            first.and(second),
            Err(ManagementError::Unauthorized)
        ));
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_administrators WHERE active=1")
                .fetch_one(f.service.store().pool())
                .await
                .unwrap();
        assert_eq!(count, 1);
    }
}
