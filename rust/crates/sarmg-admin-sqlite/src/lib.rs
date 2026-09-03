//! Transactional persistent implementation of the Foundation administrator store.

use sarmg_admin_core::{
    AdministratorRecord, AdministratorStore, DIGEST_BYTES, Identifier, LoginSuccess,
    SESSIONS_GLOBAL, SESSIONS_PER_ADMINISTRATOR, SecurityAction, SecurityAuditEvent,
    SessionAndAdministrator, SessionRecord,
};
use sqlx::{Row, Sqlite, SqlitePool, Transaction};
use thiserror::Error;

pub const ADMIN_PERSISTENT_DDL: &str = include_str!("../../../../schemas/admin/persistent-v1.sql");

#[derive(Clone, Debug)]
pub struct SqliteAdministratorStore {
    pool: SqlitePool,
}

impl SqliteAdministratorStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait::async_trait]
impl AdministratorStore for SqliteAdministratorStore {
    type StoreError = Error;

    async fn administrator_count(&self) -> Result<u64, Error> {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sarmg_administrators")
            .fetch_one(&self.pool)
            .await?;
        u64::try_from(count).map_err(|_| Error::IntegerRange)
    }

    async fn administrator_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AdministratorRecord>, Error> {
        sarmg_admin_auth::require_canonical_administrator_username(username)?;
        let row = sqlx::query(
            "SELECT administrator_id, username, password_hash, active, session_version, \
                    created_at_micros, updated_at_micros, last_login_at_micros \
             FROM _sarmg_administrators WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await?;
        row.map(|value| administrator_from_row(&value)).transpose()
    }

    async fn session_by_token_hash(
        &self,
        token_hash: [u8; DIGEST_BYTES],
    ) -> Result<Option<SessionAndAdministrator>, Error> {
        let row = sqlx::query(
            "SELECT s.session_id, s.administrator_id, s.token_hash, s.csrf_hash, \
                    s.administrator_session_version, s.created_at_micros, s.last_seen_at_micros, \
                    s.idle_expires_at_micros, s.absolute_expires_at_micros, s.revoked_at_micros, \
                    a.administrator_id, a.username, a.password_hash, a.active, a.session_version, \
                    a.created_at_micros, a.updated_at_micros, a.last_login_at_micros \
             FROM _sarmg_admin_sessions s \
             JOIN _sarmg_administrators a ON a.administrator_id = s.administrator_id \
             WHERE s.token_hash = ?",
        )
        .bind(token_hash.as_slice())
        .fetch_optional(&self.pool)
        .await?;
        row.map(|value| {
            Ok(SessionAndAdministrator {
                session: session_from_row(&value, 0)?,
                administrator: administrator_from_row_offset(&value, 10)?,
            })
        })
        .transpose()
    }

    async fn create_administrator(
        &self,
        administrator: AdministratorRecord,
        event: SecurityAuditEvent,
    ) -> Result<(), Error> {
        administrator.validate()?;
        require_action(&event, SecurityAction::AdministratorCreated)?;
        let mut transaction = self.pool.begin().await?;
        sqlx::query(
            "INSERT INTO _sarmg_administrators(\
                administrator_id, username, password_hash, active, session_version, \
                created_at_micros, updated_at_micros, last_login_at_micros\
             ) VALUES(?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(administrator.administrator_id.as_str())
        .bind(&administrator.username)
        .bind(&administrator.password_hash)
        .bind(i64::from(administrator.active))
        .bind(to_i64(administrator.session_version)?)
        .bind(to_i64(administrator.created_at_micros)?)
        .bind(to_i64(administrator.updated_at_micros)?)
        .bind(optional_i64(administrator.last_login_at_micros)?)
        .execute(&mut *transaction)
        .await?;
        insert_audit(&mut transaction, &event).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn change_password(
        &self,
        administrator_id: &Identifier,
        password_hash: &str,
        now_micros: u64,
        events: [SecurityAuditEvent; 2],
    ) -> Result<(), Error> {
        sarmg_admin_auth::require_current_password_hash(password_hash)?;
        require_action(&events[0], SecurityAction::AdministratorPasswordChanged)?;
        require_action(&events[1], SecurityAction::AdministratorSessionsRevoked)?;
        let mut transaction = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE _sarmg_administrators SET password_hash=?, session_version=session_version+1, updated_at_micros=? WHERE administrator_id=?",
        )
        .bind(password_hash)
        .bind(to_i64(now_micros)?)
        .bind(administrator_id.as_str())
        .execute(&mut *transaction)
        .await?;
        require_changed(changed.rows_affected(), administrator_id)?;
        revoke_administrator_sessions(&mut transaction, administrator_id, now_micros).await?;
        insert_audit(&mut transaction, &events[0]).await?;
        insert_audit(&mut transaction, &events[1]).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn disable_administrator(
        &self,
        administrator_id: &Identifier,
        now_micros: u64,
        events: [SecurityAuditEvent; 2],
    ) -> Result<(), Error> {
        require_action(&events[0], SecurityAction::AdministratorDisabled)?;
        require_action(&events[1], SecurityAction::AdministratorSessionsRevoked)?;
        let mut transaction = self.pool.begin().await?;
        let changed = sqlx::query(
            "UPDATE _sarmg_administrators SET active=0, session_version=session_version+1, updated_at_micros=? WHERE administrator_id=? AND active=1",
        )
        .bind(to_i64(now_micros)?)
        .bind(administrator_id.as_str())
        .execute(&mut *transaction)
        .await?;
        require_changed(changed.rows_affected(), administrator_id)?;
        revoke_administrator_sessions(&mut transaction, administrator_id, now_micros).await?;
        insert_audit(&mut transaction, &events[0]).await?;
        insert_audit(&mut transaction, &events[1]).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn commit_login_success(&self, login: LoginSuccess) -> Result<(), Error> {
        login.session.validate()?;
        require_action(&login.session_created_event, SecurityAction::SessionCreated)?;
        require_action(&login.login_succeeded_event, SecurityAction::LoginSucceeded)?;
        let mut transaction = self.pool.begin().await?;
        let current: Option<(i64, i64)> = sqlx::query_as(
            "SELECT active, session_version FROM _sarmg_administrators WHERE administrator_id=?",
        )
        .bind(login.session.administrator_id.as_str())
        .fetch_optional(&mut *transaction)
        .await?;
        let Some((active, version)) = current else {
            return Err(Error::AdministratorNotFound(
                login.session.administrator_id.to_string(),
            ));
        };
        if active != 1
            || u64::try_from(version).ok() != Some(login.session.administrator_session_version)
        {
            return Err(Error::AdministratorNotEligible);
        }
        insert_session(&mut transaction, &login.session).await?;
        enforce_session_caps(
            &mut transaction,
            &login.session,
            login.session.created_at_micros,
        )
        .await?;
        sqlx::query("UPDATE _sarmg_administrators SET last_login_at_micros=?, updated_at_micros=? WHERE administrator_id=?")
            .bind(to_i64(login.session.created_at_micros)?)
            .bind(to_i64(login.session.created_at_micros)?)
            .bind(login.session.administrator_id.as_str())
            .execute(&mut *transaction).await?;
        insert_audit(&mut transaction, &login.session_created_event).await?;
        insert_audit(&mut transaction, &login.login_succeeded_event).await?;
        transaction.commit().await?;
        Ok(())
    }

    async fn rotate_session_csrf(
        &self,
        session_id: &Identifier,
        csrf_hash: [u8; DIGEST_BYTES],
        now_micros: u64,
        idle_expires_at_micros: u64,
    ) -> Result<(), Error> {
        let changed = sqlx::query(
            "UPDATE _sarmg_admin_sessions SET csrf_hash=?, last_seen_at_micros=?, idle_expires_at_micros=? \
             WHERE session_id=? AND revoked_at_micros IS NULL AND idle_expires_at_micros>? AND absolute_expires_at_micros>?",
        )
        .bind(csrf_hash.as_slice())
        .bind(to_i64(now_micros)?)
        .bind(to_i64(idle_expires_at_micros)?)
        .bind(session_id.as_str())
        .bind(to_i64(now_micros)?)
        .bind(to_i64(now_micros)?)
        .execute(&self.pool).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::SessionNotActive);
        }
        Ok(())
    }

    async fn revoke_session(
        &self,
        session_id: &Identifier,
        now_micros: u64,
        event: SecurityAuditEvent,
    ) -> Result<(), Error> {
        require_action(&event, SecurityAction::SessionRevoked)?;
        let mut transaction = self.pool.begin().await?;
        let changed = sqlx::query("UPDATE _sarmg_admin_sessions SET revoked_at_micros=? WHERE session_id=? AND revoked_at_micros IS NULL")
            .bind(to_i64(now_micros)?).bind(session_id.as_str()).execute(&mut *transaction).await?;
        if changed.rows_affected() != 1 {
            return Err(Error::SessionNotActive);
        }
        insert_audit(&mut transaction, &event).await?;
        transaction.commit().await?;
        Ok(())
    }
}

fn administrator_from_row(row: &sqlx::sqlite::SqliteRow) -> Result<AdministratorRecord, Error> {
    administrator_from_row_offset(row, 0)
}

fn administrator_from_row_offset(
    row: &sqlx::sqlite::SqliteRow,
    offset: usize,
) -> Result<AdministratorRecord, Error> {
    let active: i64 = row.try_get(offset + 3)?;
    if !matches!(active, 0 | 1) {
        return Err(Error::InvalidStoredRecord);
    }
    let record = AdministratorRecord {
        administrator_id: Identifier::new(row.try_get::<String, _>(offset)?)?,
        username: row.try_get(offset + 1)?,
        password_hash: row.try_get(offset + 2)?,
        active: active == 1,
        session_version: from_i64(row.try_get(offset + 4)?)?,
        created_at_micros: from_i64(row.try_get(offset + 5)?)?,
        updated_at_micros: from_i64(row.try_get(offset + 6)?)?,
        last_login_at_micros: optional_u64(row.try_get(offset + 7)?)?,
    };
    record.validate()?;
    Ok(record)
}

fn session_from_row(row: &sqlx::sqlite::SqliteRow, offset: usize) -> Result<SessionRecord, Error> {
    let token: Vec<u8> = row.try_get(offset + 2)?;
    let csrf: Vec<u8> = row.try_get(offset + 3)?;
    let record = SessionRecord {
        session_id: Identifier::new(row.try_get::<String, _>(offset)?)?,
        administrator_id: Identifier::new(row.try_get::<String, _>(offset + 1)?)?,
        token_hash: token.try_into().map_err(|_| Error::InvalidStoredRecord)?,
        csrf_hash: csrf.try_into().map_err(|_| Error::InvalidStoredRecord)?,
        administrator_session_version: from_i64(row.try_get(offset + 4)?)?,
        created_at_micros: from_i64(row.try_get(offset + 5)?)?,
        last_seen_at_micros: from_i64(row.try_get(offset + 6)?)?,
        idle_expires_at_micros: from_i64(row.try_get(offset + 7)?)?,
        absolute_expires_at_micros: from_i64(row.try_get(offset + 8)?)?,
        revoked_at_micros: optional_u64(row.try_get(offset + 9)?)?,
    };
    record.validate()?;
    Ok(record)
}

async fn insert_session(
    transaction: &mut Transaction<'_, Sqlite>,
    session: &SessionRecord,
) -> Result<(), Error> {
    sqlx::query("INSERT INTO _sarmg_admin_sessions(session_id, administrator_id, token_hash, csrf_hash, administrator_session_version, created_at_micros, last_seen_at_micros, idle_expires_at_micros, absolute_expires_at_micros, revoked_at_micros) VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(session.session_id.as_str()).bind(session.administrator_id.as_str()).bind(session.token_hash.as_slice()).bind(session.csrf_hash.as_slice())
        .bind(to_i64(session.administrator_session_version)?).bind(to_i64(session.created_at_micros)?).bind(to_i64(session.last_seen_at_micros)?)
        .bind(to_i64(session.idle_expires_at_micros)?).bind(to_i64(session.absolute_expires_at_micros)?).bind(optional_i64(session.revoked_at_micros)?)
        .execute(&mut **transaction).await?;
    Ok(())
}

async fn enforce_session_caps(
    transaction: &mut Transaction<'_, Sqlite>,
    session: &SessionRecord,
    now: u64,
) -> Result<(), Error> {
    sqlx::query("UPDATE _sarmg_admin_sessions SET revoked_at_micros=? WHERE session_id IN (SELECT session_id FROM _sarmg_admin_sessions WHERE administrator_id=? AND revoked_at_micros IS NULL AND idle_expires_at_micros>? AND absolute_expires_at_micros>? ORDER BY created_at_micros DESC, session_id DESC LIMIT -1 OFFSET ?)")
        .bind(to_i64(now)?).bind(session.administrator_id.as_str()).bind(to_i64(now)?).bind(to_i64(now)?).bind(i64::try_from(SESSIONS_PER_ADMINISTRATOR).map_err(|_| Error::IntegerRange)?)
        .execute(&mut **transaction).await?;
    sqlx::query("UPDATE _sarmg_admin_sessions SET revoked_at_micros=? WHERE session_id IN (SELECT session_id FROM _sarmg_admin_sessions WHERE revoked_at_micros IS NULL AND idle_expires_at_micros>? AND absolute_expires_at_micros>? ORDER BY created_at_micros DESC, session_id DESC LIMIT -1 OFFSET ?)")
        .bind(to_i64(now)?).bind(to_i64(now)?).bind(to_i64(now)?).bind(i64::try_from(SESSIONS_GLOBAL).map_err(|_| Error::IntegerRange)?)
        .execute(&mut **transaction).await?;
    Ok(())
}

async fn revoke_administrator_sessions(
    transaction: &mut Transaction<'_, Sqlite>,
    administrator_id: &Identifier,
    now: u64,
) -> Result<(), Error> {
    sqlx::query("UPDATE _sarmg_admin_sessions SET revoked_at_micros=? WHERE administrator_id=? AND revoked_at_micros IS NULL")
        .bind(to_i64(now)?).bind(administrator_id.as_str()).execute(&mut **transaction).await?;
    Ok(())
}

async fn insert_audit(
    transaction: &mut Transaction<'_, Sqlite>,
    event: &SecurityAuditEvent,
) -> Result<(), Error> {
    event.validate()?;
    sqlx::query("INSERT INTO _sarmg_security_audit_events(event_id, action, outcome, actor_administrator_id, subject_digest, request_id, detail_json, occurred_at_micros) VALUES(?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(event.event_id.as_str()).bind(event.action.as_str()).bind(event.outcome.as_str())
        .bind(event.actor_administrator_id.as_ref().map(Identifier::as_str)).bind(event.subject_digest.as_ref().map(<[u8; DIGEST_BYTES]>::as_slice))
        .bind(event.request_id.as_deref()).bind(&event.detail_json).bind(to_i64(event.occurred_at_micros)?)
        .execute(&mut **transaction).await?;
    Ok(())
}

fn require_action(event: &SecurityAuditEvent, expected: SecurityAction) -> Result<(), Error> {
    event.validate()?;
    if event.action == expected {
        Ok(())
    } else {
        Err(Error::WrongAuditAction {
            expected,
            actual: event.action,
        })
    }
}
fn require_changed(rows: u64, identifier: &Identifier) -> Result<(), Error> {
    if rows == 1 {
        Ok(())
    } else {
        Err(Error::AdministratorNotFound(identifier.to_string()))
    }
}
fn to_i64(value: u64) -> Result<i64, Error> {
    i64::try_from(value).map_err(|_| Error::IntegerRange)
}
fn from_i64(value: i64) -> Result<u64, Error> {
    u64::try_from(value).map_err(|_| Error::InvalidStoredRecord)
}
fn optional_i64(value: Option<u64>) -> Result<Option<i64>, Error> {
    value.map(to_i64).transpose()
}
fn optional_u64(value: Option<i64>) -> Result<Option<u64>, Error> {
    value.map(from_i64).transpose()
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Core(#[from] sarmg_admin_core::Error),
    #[error(transparent)]
    Authentication(#[from] sarmg_admin_auth::Error),
    #[error("administrator not found: {0}")]
    AdministratorNotFound(String),
    #[error("administrator is inactive or its session revision changed")]
    AdministratorNotEligible,
    #[error("session is absent, revoked, or expired")]
    SessionNotActive,
    #[error("stored administrator/session data violates the current contract")]
    InvalidStoredRecord,
    #[error("integer is outside the SQLite signed 64-bit range")]
    IntegerRange,
    #[error("audit action differs: expected {expected:?}, found {actual:?}")]
    WrongAuditAction {
        expected: SecurityAction,
        actual: SecurityAction,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarmg_admin_core::{AdministratorStore, AuditOutcome};

    fn event(
        id: &str,
        action: SecurityAction,
        administrator_id: &Identifier,
    ) -> SecurityAuditEvent {
        SecurityAuditEvent {
            event_id: Identifier::new(id).unwrap(),
            action,
            outcome: AuditOutcome::Success,
            actor_administrator_id: Some(administrator_id.clone()),
            subject_digest: None,
            request_id: Some("request-1".into()),
            detail_json: "{}".into(),
            occurred_at_micros: 10,
        }
    }

    fn session(administrator_id: &Identifier, id: &str, byte: u8) -> SessionRecord {
        SessionRecord {
            session_id: Identifier::new(id).unwrap(),
            administrator_id: administrator_id.clone(),
            token_hash: [byte; 32],
            csrf_hash: [byte.wrapping_add(1); 32],
            administrator_session_version: 1,
            created_at_micros: 10,
            last_seen_at_micros: 10,
            idle_expires_at_micros: 100,
            absolute_expires_at_micros: 200,
            revoked_at_micros: None,
        }
    }

    #[tokio::test]
    async fn mutations_and_audit_are_atomic() -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let pool = sarmg_sqlite::create_if_missing(
            directory.path().join("admin.sqlite3"),
            sarmg_sqlite::PoolOptions::new(1),
        )
        .await?;
        sqlx::raw_sql(ADMIN_PERSISTENT_DDL).execute(&pool).await?;
        let store = SqliteAdministratorStore::new(pool.clone());
        let administrator_id = Identifier::new("administrator-1")?;
        store
            .create_administrator(
                AdministratorRecord {
                    administrator_id: administrator_id.clone(),
                    username: "admin".into(),
                    password_hash: sarmg_admin_auth::hash_password("correct horse battery")?,
                    active: true,
                    session_version: 1,
                    created_at_micros: 1,
                    updated_at_micros: 1,
                    last_login_at_micros: None,
                },
                event(
                    "event-created",
                    SecurityAction::AdministratorCreated,
                    &administrator_id,
                ),
            )
            .await?;

        let rolled_back = session(&administrator_id, "session-rollback", 3);
        assert!(
            store
                .commit_login_success(LoginSuccess {
                    session: rolled_back.clone(),
                    session_created_event: event(
                        "event-created",
                        SecurityAction::SessionCreated,
                        &administrator_id
                    ),
                    login_succeeded_event: event(
                        "event-never",
                        SecurityAction::LoginSucceeded,
                        &administrator_id
                    ),
                })
                .await
                .is_err()
        );
        assert!(
            store
                .session_by_token_hash(rolled_back.token_hash)
                .await?
                .is_none()
        );

        let active = session(&administrator_id, "session-active", 7);
        store
            .commit_login_success(LoginSuccess {
                session: active.clone(),
                session_created_event: event(
                    "event-session",
                    SecurityAction::SessionCreated,
                    &administrator_id,
                ),
                login_succeeded_event: event(
                    "event-login",
                    SecurityAction::LoginSucceeded,
                    &administrator_id,
                ),
            })
            .await?;
        assert_eq!(
            store
                .session_by_token_hash(active.token_hash)
                .await?
                .unwrap()
                .session,
            active
        );
        let audit_count: i64 =
            sqlx::query_scalar("SELECT count(*) FROM _sarmg_security_audit_events")
                .fetch_one(&pool)
                .await?;
        assert_eq!(audit_count, 3);
        Ok(())
    }
}
