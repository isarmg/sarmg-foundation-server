//! Static administrator configuration and bounded process-local sessions.
//!
//! Configuration mutation is intentionally unsupported. Restarting the
//! process drops every browser session by construction.

use sarmg_admin_core::{
    AdministratorRecord, AdministratorStore, DIGEST_BYTES, Identifier, LoginSuccess,
    SESSIONS_GLOBAL, SESSIONS_PER_ADMINISTRATOR, SecurityAction, SecurityAuditEvent,
    SessionAndAdministrator, SessionRecord,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};
use thiserror::Error;

#[derive(Debug)]
struct State {
    administrators: HashMap<String, AdministratorRecord>,
    sessions: HashMap<[u8; DIGEST_BYTES], SessionRecord>,
    audit_events: Vec<SecurityAuditEvent>,
}

#[derive(Debug)]
pub struct StaticAdministratorStore {
    state: Mutex<State>,
}

impl StaticAdministratorStore {
    pub fn new(
        administrators: impl IntoIterator<Item = AdministratorRecord>,
    ) -> Result<Self, Error> {
        let mut by_username = HashMap::new();
        let mut identifiers = HashSet::new();
        for administrator in administrators {
            if by_username.len() >= sarmg_admin_core::STATIC_ADMINISTRATORS_MAX {
                return Err(Error::TooManyAdministrators);
            }
            administrator.validate()?;
            if !administrator.active {
                return Err(Error::InactiveConfiguredAdministrator);
            }
            let username = administrator.username.clone();
            if !identifiers.insert(administrator.administrator_id.clone()) {
                return Err(Error::DuplicateIdentifier);
            }
            if by_username
                .insert(username.clone(), administrator)
                .is_some()
            {
                return Err(Error::DuplicateUsername(username));
            }
        }
        if by_username.is_empty() {
            return Err(Error::NoAdministrators);
        }
        Ok(Self {
            state: Mutex::new(State {
                administrators: by_username,
                sessions: HashMap::new(),
                audit_events: Vec::new(),
            }),
        })
    }

    pub fn active_session_count(&self) -> Result<usize, Error> {
        Ok(self
            .lock()?
            .sessions
            .values()
            .filter(|session| session.revoked_at_micros.is_none())
            .count())
    }

    pub fn audit_events(&self) -> Result<Vec<SecurityAuditEvent>, Error> {
        Ok(self.lock()?.audit_events.clone())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, State>, Error> {
        self.state.lock().map_err(|_| Error::StatePoisoned)
    }
}

#[async_trait::async_trait]
impl AdministratorStore for StaticAdministratorStore {
    type StoreError = Error;

    fn supports_management(&self) -> bool {
        false
    }

    async fn list_administrators(&self, _: u32, _: u64) -> Result<Vec<AdministratorRecord>, Error> {
        Err(Error::ConfigurationMutationUnsupported)
    }

    async fn manage_administrator(
        &self,
        _: &sarmg_admin_core::AdministratorManagementContext,
        _: sarmg_admin_core::AdministratorMutation,
    ) -> Result<(), sarmg_admin_core::ManagementError<Error>> {
        Err(sarmg_admin_core::ManagementError::Unsupported)
    }

    async fn administrator_count(&self) -> Result<u64, Error> {
        Ok(u64::try_from(self.lock()?.administrators.len())
            .expect("supported targets represent usize within u64"))
    }

    async fn administrator_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AdministratorRecord>, Error> {
        sarmg_admin_auth::require_canonical_administrator_username(username)?;
        Ok(self.lock()?.administrators.get(username).cloned())
    }

    async fn session_by_token_hash(
        &self,
        token_hash: [u8; DIGEST_BYTES],
    ) -> Result<Option<SessionAndAdministrator>, Error> {
        let state = self.lock()?;
        let Some(session) = state.sessions.get(&token_hash).cloned() else {
            return Ok(None);
        };
        let administrator = state
            .administrators
            .values()
            .find(|administrator| administrator.administrator_id == session.administrator_id)
            .cloned()
            .ok_or(Error::InvalidSessionAdministrator)?;
        Ok(Some(SessionAndAdministrator {
            session,
            administrator,
        }))
    }

    async fn create_administrator(
        &self,
        _: AdministratorRecord,
        _: SecurityAuditEvent,
    ) -> Result<(), Error> {
        Err(Error::ConfigurationMutationUnsupported)
    }

    async fn change_password(
        &self,
        _: &Identifier,
        _: &str,
        _: u64,
        _: [SecurityAuditEvent; 2],
    ) -> Result<(), Error> {
        Err(Error::ConfigurationMutationUnsupported)
    }

    async fn commit_login_success(&self, login: LoginSuccess) -> Result<(), Error> {
        login.session.validate()?;
        require_action(&login.session_created_event, SecurityAction::SessionCreated)?;
        require_action(&login.login_succeeded_event, SecurityAction::LoginSucceeded)?;
        let mut state = self.lock()?;
        let administrator = state
            .administrators
            .values_mut()
            .find(|administrator| administrator.administrator_id == login.session.administrator_id)
            .ok_or(Error::InvalidSessionAdministrator)?;
        if !administrator.active
            || administrator.session_version != login.session.administrator_session_version
        {
            return Err(Error::InvalidSessionAdministrator);
        }
        administrator.last_login_at_micros = Some(login.session.created_at_micros);
        administrator.updated_at_micros = login.session.created_at_micros;
        let now = login.session.created_at_micros;
        state
            .sessions
            .insert(login.session.token_hash, login.session.clone());
        prune_sessions(&mut state.sessions, &login.session.administrator_id, now);
        state.audit_events.push(login.session_created_event);
        state.audit_events.push(login.login_succeeded_event);
        Ok(())
    }

    async fn rotate_session_csrf(
        &self,
        session_id: &Identifier,
        expected_csrf_hash: [u8; DIGEST_BYTES],
        csrf_hash: [u8; DIGEST_BYTES],
        now_micros: u64,
        idle_expires_at_micros: u64,
    ) -> Result<bool, Error> {
        let mut state = self.lock()?;
        let Some(session) = state
            .sessions
            .values_mut()
            .find(|session| &session.session_id == session_id)
        else {
            return Ok(false);
        };
        if session.revoked_at_micros.is_some()
            || session.csrf_hash != expected_csrf_hash
            || now_micros < session.last_seen_at_micros
            || idle_expires_at_micros <= now_micros
            || now_micros >= session.idle_expires_at_micros
            || now_micros >= session.absolute_expires_at_micros
            || idle_expires_at_micros > session.absolute_expires_at_micros
        {
            return Ok(false);
        }
        session.csrf_hash = csrf_hash;
        session.last_seen_at_micros = now_micros;
        session.idle_expires_at_micros = idle_expires_at_micros;
        Ok(true)
    }

    async fn revoke_session(
        &self,
        session_id: &Identifier,
        now_micros: u64,
        event: SecurityAuditEvent,
    ) -> Result<(), Error> {
        require_action(&event, SecurityAction::SessionRevoked)?;
        let mut state = self.lock()?;
        let session = state
            .sessions
            .values_mut()
            .find(|session| &session.session_id == session_id)
            .ok_or(Error::SessionNotActive)?;
        if session.revoked_at_micros.is_some() {
            return Err(Error::SessionNotActive);
        }
        session.revoked_at_micros = Some(now_micros);
        state.audit_events.push(event);
        Ok(())
    }
}

fn prune_sessions(
    sessions: &mut HashMap<[u8; DIGEST_BYTES], SessionRecord>,
    administrator_id: &Identifier,
    now: u64,
) {
    let mut administrator_keys: Vec<_> = sessions
        .iter()
        .filter(|(_, session)| {
            &session.administrator_id == administrator_id && is_active(session, now)
        })
        .map(|(key, session)| {
            (
                *key,
                session.created_at_micros,
                session.session_id.as_str().to_owned(),
            )
        })
        .collect();
    administrator_keys
        .sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.2.cmp(&left.2)));
    for (key, _, _) in administrator_keys
        .into_iter()
        .skip(SESSIONS_PER_ADMINISTRATOR)
    {
        if let Some(session) = sessions.get_mut(&key) {
            session.revoked_at_micros = Some(now);
        }
    }
    let mut global_keys: Vec<_> = sessions
        .iter()
        .filter(|(_, session)| is_active(session, now))
        .map(|(key, session)| {
            (
                *key,
                session.created_at_micros,
                session.session_id.as_str().to_owned(),
            )
        })
        .collect();
    global_keys.sort_by(|left, right| right.1.cmp(&left.1).then_with(|| right.2.cmp(&left.2)));
    for (key, _, _) in global_keys.into_iter().skip(SESSIONS_GLOBAL) {
        if let Some(session) = sessions.get_mut(&key) {
            session.revoked_at_micros = Some(now);
        }
    }
}

fn is_active(session: &SessionRecord, now: u64) -> bool {
    session.revoked_at_micros.is_none()
        && session.idle_expires_at_micros > now
        && session.absolute_expires_at_micros > now
}

fn require_action(event: &SecurityAuditEvent, expected: SecurityAction) -> Result<(), Error> {
    event.validate()?;
    if event.action == expected {
        Ok(())
    } else {
        Err(Error::WrongAuditAction)
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Core(#[from] sarmg_admin_core::Error),
    #[error(transparent)]
    Authentication(#[from] sarmg_admin_auth::Error),
    #[error("static administrator configuration must contain at least one administrator")]
    NoAdministrators,
    #[error("static administrator configuration exceeds the platform capacity")]
    TooManyAdministrators,
    #[error("static administrator identifiers must be unique")]
    DuplicateIdentifier,
    #[error("static administrator must be active")]
    InactiveConfiguredAdministrator,
    #[error("duplicate static administrator username: {0}")]
    DuplicateUsername(String),
    #[error("static administrator configuration cannot be mutated through the Web API")]
    ConfigurationMutationUnsupported,
    #[error("session references no current static administrator")]
    InvalidSessionAdministrator,
    #[error("session is absent, revoked, or expired")]
    SessionNotActive,
    #[error("security audit action differs from the required action")]
    WrongAuditAction,
    #[error("in-memory administrator state lock was poisoned")]
    StatePoisoned,
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarmg_admin_core::{
        AdministratorService, AdministratorStore, AuditOutcome, LoginContext, ServiceError,
    };

    #[test]
    fn static_identifiers_and_account_capacity_are_platform_invariants() {
        let administrator = AdministratorRecord {
            administrator_id: Identifier::new("first").unwrap(),
            username: "admin".into(),
            password_hash: sarmg_admin_auth::hash_password("correct horse battery").unwrap(),
            active: true,
            session_version: 1,
            created_at_micros: 1,
            updated_at_micros: 1,
            last_login_at_micros: None,
        };
        let mut second = administrator.clone();
        second.username = "secondary".into();
        assert!(matches!(
            StaticAdministratorStore::new([administrator.clone(), second]),
            Err(Error::DuplicateIdentifier)
        ));
        let records = (0..sarmg_admin_core::STATIC_ADMINISTRATORS_MAX)
            .map(|index| {
                let username = format!("admin-{index}");
                AdministratorRecord {
                    administrator_id: Identifier::new(username.clone()).unwrap(),
                    username,
                    ..administrator.clone()
                }
            })
            .collect::<Vec<_>>();
        assert!(StaticAdministratorStore::new(records.clone()).is_ok());
        assert!(matches!(
            StaticAdministratorStore::new(records.into_iter().chain([administrator])),
            Err(Error::TooManyAdministrators)
        ));
    }

    fn audit(
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
            request_id: None,
            detail_json: "{}".into(),
            occurred_at_micros: 2,
        }
    }

    #[tokio::test]
    async fn static_accounts_support_sessions_but_not_mutation()
    -> Result<(), Box<dyn std::error::Error>> {
        let administrator_id = Identifier::new("static-admin")?;
        let administrator = AdministratorRecord {
            administrator_id: administrator_id.clone(),
            username: "admin".into(),
            password_hash: sarmg_admin_auth::hash_password("correct horse battery")?,
            active: true,
            session_version: 1,
            created_at_micros: 1,
            updated_at_micros: 1,
            last_login_at_micros: None,
        };
        let store = StaticAdministratorStore::new([administrator])?;
        let session = SessionRecord {
            session_id: Identifier::new("static-session")?,
            administrator_id: administrator_id.clone(),
            token_hash: [5; 32],
            csrf_hash: [6; 32],
            administrator_session_version: 1,
            created_at_micros: 2,
            last_seen_at_micros: 2,
            idle_expires_at_micros: 100,
            absolute_expires_at_micros: 200,
            revoked_at_micros: None,
        };
        store
            .commit_login_success(LoginSuccess {
                session: session.clone(),
                session_created_event: audit(
                    "session-event",
                    SecurityAction::SessionCreated,
                    &administrator_id,
                ),
                login_succeeded_event: audit(
                    "login-event",
                    SecurityAction::LoginSucceeded,
                    &administrator_id,
                ),
            })
            .await?;
        assert_eq!(store.active_session_count()?, 1);
        assert_eq!(
            store.session_by_token_hash([5; 32]).await?.unwrap().session,
            session
        );
        assert!(
            store
                .rotate_session_csrf(&session.session_id, [6; 32], [7; 32], 3, 100)
                .await?
        );
        assert!(
            !store
                .rotate_session_csrf(&session.session_id, [6; 32], [6; 32], 4, 100)
                .await?
        );
        assert!(
            !store
                .rotate_session_csrf(&session.session_id, [6; 32], [8; 32], 4, 100)
                .await?
        );
        assert!(
            !store
                .rotate_session_csrf(&session.session_id, [7; 32], [7; 32], 2, 100)
                .await?
        );
        assert_eq!(
            store
                .session_by_token_hash([5; 32])
                .await?
                .unwrap()
                .session
                .csrf_hash,
            [7; 32]
        );
        assert!(!store.supports_management());
        assert!(matches!(
            store.list_administrators(10, 0).await,
            Err(Error::ConfigurationMutationUnsupported)
        ));
        assert!(matches!(
            store
                .manage_administrator(
                    &sarmg_admin_core::AdministratorManagementContext {
                        identity: sarmg_admin_core::AuthenticatedIdentity {
                            administrator_id: administrator_id.clone(),
                            username: "admin".into(),
                            session_id: session.session_id.clone(),
                            csrf_hash: session.csrf_hash,
                        },
                        request_id: None,
                        now_micros: 3,
                    },
                    sarmg_admin_core::AdministratorMutation::Disable { administrator_id },
                )
                .await,
            Err(sarmg_admin_core::ManagementError::Unsupported)
        ));
        Ok(())
    }

    #[tokio::test]
    async fn service_equalizes_failures_and_commits_success()
    -> Result<(), Box<dyn std::error::Error>> {
        let administrator_id = Identifier::new("service-admin")?;
        let store = StaticAdministratorStore::new([AdministratorRecord {
            administrator_id,
            username: "admin".into(),
            password_hash: sarmg_admin_auth::hash_password("correct horse battery")?,
            active: true,
            session_version: 1,
            created_at_micros: 1,
            updated_at_micros: 1,
            last_login_at_micros: None,
        }])?;
        let service = AdministratorService::new(store);
        let context = LoginContext {
            source: "127.0.0.1".into(),
            request_id: Some("request-1".into()),
            now_micros: 10,
        };
        assert!(matches!(
            service.login("missing", "wrong password!", &context).await,
            Err(ServiceError::InvalidCredentials)
        ));
        assert!(matches!(
            service.login("admin", "wrong password!", &context).await,
            Err(ServiceError::InvalidCredentials)
        ));
        let authenticated = service
            .login(" ADMIN ", "correct horse battery", &context)
            .await?;
        assert_eq!(authenticated.administrator.username, "admin");
        assert!(sarmg_admin_auth::is_token_shape(
            &authenticated.session_token
        ));
        assert_eq!(service.store().active_session_count()?, 1);
        assert_eq!(service.store().audit_events()?.len(), 2);
        Ok(())
    }
}
