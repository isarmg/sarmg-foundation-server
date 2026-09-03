//! Framework- and storage-independent administrator control-plane contract.

use sarmg_admin_auth::{
    AdministratorOriginMode, require_canonical_administrator_username,
    require_current_password_hash,
};
use std::{
    collections::{HashMap, VecDeque},
    fmt,
    sync::{Arc, Mutex},
    time::Duration,
};
use thiserror::Error;
use tokio::sync::Semaphore;

pub const POLICY_REVISION: u32 = 1;
pub const SESSION_IDLE_MICROS: u64 = 30 * 60 * 1_000_000;
pub const SESSION_ABSOLUTE_MICROS: u64 = 12 * 60 * 60 * 1_000_000;
pub const SESSIONS_PER_ADMINISTRATOR: usize = 32;
pub const SESSIONS_GLOBAL: usize = 1_024;
pub const LOGIN_WINDOW_MICROS: u64 = 5 * 60 * 1_000_000;
pub const FAILURES_PER_SOURCE: usize = 20;
pub const FAILURES_PER_ACCOUNT: usize = 10;
pub const ARGON2_CONCURRENCY: usize = 2;
pub const ARGON2_ACQUIRE_TIMEOUT_MICROS: u64 = 2 * 1_000_000;
pub const SESSION_LAST_SEEN_WRITE_INTERVAL_MICROS: u64 = 60 * 1_000_000;
pub const DIGEST_BYTES: usize = 32;
pub const IDENTIFIER_MAX_BYTES: usize = 64;
pub const MAX_LOGIN_TRACKED_KEYS: usize = 4_096;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AdministratorPolicyV1;

impl AdministratorPolicyV1 {
    pub const fn revision(self) -> u32 {
        POLICY_REVISION
    }
    pub const fn session_idle_micros(self) -> u64 {
        SESSION_IDLE_MICROS
    }
    pub const fn session_absolute_micros(self) -> u64 {
        SESSION_ABSOLUTE_MICROS
    }
    pub const fn sessions_per_administrator(self) -> usize {
        SESSIONS_PER_ADMINISTRATOR
    }
    pub const fn sessions_global(self) -> usize {
        SESSIONS_GLOBAL
    }
    pub const fn login_window_micros(self) -> u64 {
        LOGIN_WINDOW_MICROS
    }
    pub const fn failures_per_source(self) -> usize {
        FAILURES_PER_SOURCE
    }
    pub const fn failures_per_account(self) -> usize {
        FAILURES_PER_ACCOUNT
    }
    pub const fn argon2_concurrency(self) -> usize {
        ARGON2_CONCURRENCY
    }
    pub const fn argon2_acquire_timeout_micros(self) -> u64 {
        ARGON2_ACQUIRE_TIMEOUT_MICROS
    }
    pub const fn last_seen_write_interval_micros(self) -> u64 {
        SESSION_LAST_SEEN_WRITE_INTERVAL_MICROS
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Identifier(String);

impl Identifier {
    pub fn new(value: impl Into<String>) -> Result<Self, Error> {
        let value = value.into();
        if value.is_empty()
            || value.len() > IDENTIFIER_MAX_BYTES
            || !value.is_ascii()
            || value
                .bytes()
                .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace())
        {
            return Err(Error::InvalidIdentifier);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdministratorRecord {
    pub administrator_id: Identifier,
    pub username: String,
    pub password_hash: String,
    pub active: bool,
    pub session_version: u64,
    pub created_at_micros: u64,
    pub updated_at_micros: u64,
    pub last_login_at_micros: Option<u64>,
}

impl AdministratorRecord {
    pub fn validate(&self) -> Result<(), Error> {
        require_canonical_administrator_username(&self.username)?;
        require_current_password_hash(&self.password_hash)?;
        require_persistable_time(self.created_at_micros)?;
        require_persistable_time(self.updated_at_micros)?;
        if self.session_version == 0
            || self.updated_at_micros < self.created_at_micros
            || self
                .last_login_at_micros
                .is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(Error::InvalidAdministratorRecord);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub session_id: Identifier,
    pub administrator_id: Identifier,
    pub token_hash: [u8; DIGEST_BYTES],
    pub csrf_hash: [u8; DIGEST_BYTES],
    pub administrator_session_version: u64,
    pub created_at_micros: u64,
    pub last_seen_at_micros: u64,
    pub idle_expires_at_micros: u64,
    pub absolute_expires_at_micros: u64,
    pub revoked_at_micros: Option<u64>,
}

impl SessionRecord {
    pub fn validate(&self) -> Result<(), Error> {
        for value in [
            self.created_at_micros,
            self.last_seen_at_micros,
            self.idle_expires_at_micros,
            self.absolute_expires_at_micros,
        ] {
            require_persistable_time(value)?;
        }
        if self.administrator_session_version == 0
            || self.last_seen_at_micros < self.created_at_micros
            || self.idle_expires_at_micros <= self.created_at_micros
            || self.absolute_expires_at_micros < self.idle_expires_at_micros
            || self
                .revoked_at_micros
                .is_some_and(|value| value > i64::MAX as u64)
        {
            return Err(Error::InvalidSessionRecord);
        }
        Ok(())
    }

    pub fn status(&self, administrator: &AdministratorRecord, now_micros: u64) -> SessionStatus {
        if self.revoked_at_micros.is_some() {
            SessionStatus::Revoked
        } else if !administrator.active {
            SessionStatus::AdministratorInactive
        } else if self.administrator_session_version != administrator.session_version {
            SessionStatus::AdministratorVersionChanged
        } else if now_micros >= self.absolute_expires_at_micros {
            SessionStatus::AbsoluteExpired
        } else if now_micros >= self.idle_expires_at_micros {
            SessionStatus::IdleExpired
        } else {
            SessionStatus::Active
        }
    }

    pub fn should_write_last_seen(&self, now_micros: u64) -> bool {
        now_micros.saturating_sub(self.last_seen_at_micros)
            >= SESSION_LAST_SEEN_WRITE_INTERVAL_MICROS
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SessionStatus {
    Active,
    Revoked,
    AdministratorInactive,
    AdministratorVersionChanged,
    IdleExpired,
    AbsoluteExpired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditOutcome {
    Success,
    Failure,
}

impl AuditOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecurityAction {
    AdministratorCreated,
    AdministratorDisabled,
    AdministratorPasswordChanged,
    AdministratorSessionsRevoked,
    SessionCreated,
    SessionRevoked,
    LoginSucceeded,
}

impl SecurityAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AdministratorCreated => "administrator.created",
            Self::AdministratorDisabled => "administrator.disabled",
            Self::AdministratorPasswordChanged => "administrator.password_changed",
            Self::AdministratorSessionsRevoked => "administrator.sessions_revoked",
            Self::SessionCreated => "session.created",
            Self::SessionRevoked => "session.revoked",
            Self::LoginSucceeded => "login.succeeded",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SecurityAuditEvent {
    pub event_id: Identifier,
    pub action: SecurityAction,
    pub outcome: AuditOutcome,
    pub actor_administrator_id: Option<Identifier>,
    pub subject_digest: Option<[u8; DIGEST_BYTES]>,
    pub request_id: Option<String>,
    pub detail_json: String,
    pub occurred_at_micros: u64,
}

impl SecurityAuditEvent {
    pub fn validate(&self) -> Result<(), Error> {
        require_persistable_time(self.occurred_at_micros)?;
        let detail: serde_json::Value =
            serde_json::from_str(&self.detail_json).map_err(|_| Error::InvalidAuditEvent)?;
        if !detail.is_object()
            || self
                .request_id
                .as_ref()
                .is_some_and(|value| value.is_empty() || value.len() > 128 || !value.is_ascii())
        {
            return Err(Error::InvalidAuditEvent);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginSuccess {
    pub session: SessionRecord,
    pub session_created_event: SecurityAuditEvent,
    pub login_succeeded_event: SecurityAuditEvent,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionAndAdministrator {
    pub session: SessionRecord,
    pub administrator: AdministratorRecord,
}

#[async_trait::async_trait]
pub trait AdministratorStore: Send + Sync {
    type StoreError: std::error::Error + Send + Sync + 'static;
    async fn administrator_count(&self) -> Result<u64, Self::StoreError>;
    async fn administrator_by_username(
        &self,
        username: &str,
    ) -> Result<Option<AdministratorRecord>, Self::StoreError>;
    async fn session_by_token_hash(
        &self,
        token_hash: [u8; DIGEST_BYTES],
    ) -> Result<Option<SessionAndAdministrator>, Self::StoreError>;
    async fn create_administrator(
        &self,
        administrator: AdministratorRecord,
        event: SecurityAuditEvent,
    ) -> Result<(), Self::StoreError>;
    async fn change_password(
        &self,
        administrator_id: &Identifier,
        password_hash: &str,
        now_micros: u64,
        events: [SecurityAuditEvent; 2],
    ) -> Result<(), Self::StoreError>;
    async fn disable_administrator(
        &self,
        administrator_id: &Identifier,
        now_micros: u64,
        events: [SecurityAuditEvent; 2],
    ) -> Result<(), Self::StoreError>;
    async fn commit_login_success(&self, login: LoginSuccess) -> Result<(), Self::StoreError>;
    async fn rotate_session_csrf(
        &self,
        session_id: &Identifier,
        csrf_hash: [u8; DIGEST_BYTES],
        now_micros: u64,
        idle_expires_at_micros: u64,
    ) -> Result<(), Self::StoreError>;
    async fn revoke_session(
        &self,
        session_id: &Identifier,
        now_micros: u64,
        event: SecurityAuditEvent,
    ) -> Result<(), Self::StoreError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoginContext {
    pub source: String,
    pub request_id: Option<String>,
    pub now_micros: u64,
}

impl LoginContext {
    pub fn validate(&self) -> Result<(), Error> {
        require_persistable_time(self.now_micros)?;
        if self.source.is_empty()
            || self.source.len() > 256
            || !self.source.is_ascii()
            || self.source.bytes().any(|byte| byte.is_ascii_control())
        {
            return Err(Error::InvalidLoginSource);
        }
        if let Some(request_id) = &self.request_id {
            sarmg_contracts::RequestId::new(request_id.clone())
                .map_err(|_| Error::InvalidRequestId)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedSession {
    pub administrator: sarmg_contracts::AdministratorSession,
    pub session_token: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuthenticatedIdentity {
    pub administrator_id: Identifier,
    pub username: String,
    pub session_id: Identifier,
    pub csrf_hash: [u8; DIGEST_BYTES],
}

#[derive(Debug, Default)]
struct LoginFailures {
    source: HashMap<[u8; DIGEST_BYTES], VecDeque<u64>>,
    account: HashMap<[u8; DIGEST_BYTES], VecDeque<u64>>,
}

/// Complete storage-independent login/session orchestration. HTTP adapters are
/// intentionally limited to wire validation and error/cookie translation.
#[derive(Debug)]
pub struct AdministratorService<Store> {
    store: Store,
    argon2_slots: Arc<Semaphore>,
    failures: Mutex<LoginFailures>,
}

impl<Store> AdministratorService<Store>
where
    Store: AdministratorStore,
{
    pub fn new(store: Store) -> Self {
        Self {
            store,
            argon2_slots: Arc::new(Semaphore::new(ARGON2_CONCURRENCY)),
            failures: Mutex::new(LoginFailures::default()),
        }
    }

    pub fn store(&self) -> &Store {
        &self.store
    }

    /// Create the first administrator when the persistent store is empty.
    /// Returns `false` without mutating the store once any administrator exists.
    pub async fn bootstrap_administrator(
        &self,
        username_candidate: &str,
        password: &str,
        now_micros: u64,
    ) -> Result<bool, ServiceError<Store::StoreError>> {
        require_persistable_time(now_micros)?;
        let username = sarmg_admin_auth::normalize_administrator_username(username_candidate)?;
        if self
            .store
            .administrator_count()
            .await
            .map_err(ServiceError::Store)?
            != 0
        {
            return Ok(false);
        }
        self.create_administrator_record(username, password, now_micros)
            .await?;
        Ok(true)
    }

    /// Change an administrator password and revoke all existing sessions in
    /// the same store transaction.
    pub async fn change_administrator_password(
        &self,
        username_candidate: &str,
        password: &str,
        now_micros: u64,
    ) -> Result<(), ServiceError<Store::StoreError>> {
        require_persistable_time(now_micros)?;
        let username = sarmg_admin_auth::normalize_administrator_username(username_candidate)?;
        let administrator = self
            .store
            .administrator_by_username(&username)
            .await
            .map_err(ServiceError::Store)?
            .ok_or(ServiceError::AdministratorNotFound)?;
        let password_hash = self.hash_password(password).await?;
        let events = [
            maintenance_event(
                SecurityAction::AdministratorPasswordChanged,
                &administrator,
                now_micros,
            )?,
            maintenance_event(
                SecurityAction::AdministratorSessionsRevoked,
                &administrator,
                now_micros,
            )?,
        ];
        self.store
            .change_password(
                &administrator.administrator_id,
                &password_hash,
                now_micros,
                events,
            )
            .await
            .map_err(ServiceError::Store)
    }

    async fn create_administrator_record(
        &self,
        username: String,
        password: &str,
        now_micros: u64,
    ) -> Result<(), ServiceError<Store::StoreError>> {
        let password_hash = self.hash_password(password).await?;
        let administrator = AdministratorRecord {
            administrator_id: Identifier::new(sarmg_admin_auth::random_token()?)?,
            username,
            password_hash,
            active: true,
            session_version: 1,
            created_at_micros: now_micros,
            updated_at_micros: now_micros,
            last_login_at_micros: None,
        };
        let event = maintenance_event(
            SecurityAction::AdministratorCreated,
            &administrator,
            now_micros,
        )?;
        self.store
            .create_administrator(administrator, event)
            .await
            .map_err(ServiceError::Store)
    }

    async fn hash_password(
        &self,
        password: &str,
    ) -> Result<String, ServiceError<Store::StoreError>> {
        sarmg_admin_auth::validate_password(password)?;
        let permit = tokio::time::timeout(
            Duration::from_micros(ARGON2_ACQUIRE_TIMEOUT_MICROS),
            self.argon2_slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| ServiceError::AuthenticationBusy)?
        .map_err(|_| ServiceError::AuthenticationBusy)?;
        let password = password.to_owned();
        tokio::task::spawn_blocking(move || {
            let result = sarmg_admin_auth::hash_password(&password);
            drop(permit);
            result
        })
        .await
        .map_err(|_| ServiceError::AuthenticationWorkerFailed)?
        .map_err(ServiceError::Authentication)
    }

    pub async fn login(
        &self,
        username_candidate: &str,
        password: &str,
        context: &LoginContext,
    ) -> Result<AuthenticatedSession, ServiceError<Store::StoreError>> {
        context.validate()?;
        let normalized =
            sarmg_admin_auth::normalize_administrator_username(username_candidate).ok();
        let source_key = sarmg_admin_auth::token_hash(&context.source);
        let account_key =
            sarmg_admin_auth::token_hash(normalized.as_deref().unwrap_or(username_candidate));
        self.require_login_capacity(source_key, account_key, context.now_micros)
            .map_err(|error| match error {
                Error::LoginRateLimited => ServiceError::LoginRateLimited,
                other => ServiceError::Policy(other),
            })?;

        let administrator = match normalized.as_deref() {
            Some(username) => self
                .store
                .administrator_by_username(username)
                .await
                .map_err(ServiceError::Store)?,
            None => None,
        };
        let encoded = administrator
            .as_ref()
            .map_or(DUMMY_PASSWORD_HASH, |value| value.password_hash.as_str());
        let permit = tokio::time::timeout(
            Duration::from_micros(ARGON2_ACQUIRE_TIMEOUT_MICROS),
            self.argon2_slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| ServiceError::AuthenticationBusy)?
        .map_err(|_| ServiceError::AuthenticationBusy)?;
        let password = password.to_owned();
        let encoded = encoded.to_owned();
        let password_matches = tokio::task::spawn_blocking(move || {
            let matches = sarmg_admin_auth::verify_password(&password, &encoded);
            drop(permit);
            matches
        })
        .await
        .map_err(|_| ServiceError::AuthenticationWorkerFailed)?;
        let Some(administrator) = administrator.filter(|value| value.active && password_matches)
        else {
            self.record_login_failure(source_key, account_key, context.now_micros)?;
            return Err(ServiceError::InvalidCredentials);
        };

        let session_token = sarmg_admin_auth::random_token()?;
        let csrf_token = sarmg_admin_auth::random_token()?;
        let idle_expires_at_micros = context
            .now_micros
            .checked_add(SESSION_IDLE_MICROS)
            .ok_or(Error::InvalidTimestamp)?;
        let absolute_expires_at_micros = context
            .now_micros
            .checked_add(SESSION_ABSOLUTE_MICROS)
            .ok_or(Error::InvalidTimestamp)?;
        let session = SessionRecord {
            session_id: Identifier::new(sarmg_admin_auth::random_token()?)?,
            administrator_id: administrator.administrator_id.clone(),
            token_hash: sarmg_admin_auth::token_hash(&session_token),
            csrf_hash: sarmg_admin_auth::token_hash(&csrf_token),
            administrator_session_version: administrator.session_version,
            created_at_micros: context.now_micros,
            last_seen_at_micros: context.now_micros,
            idle_expires_at_micros,
            absolute_expires_at_micros,
            revoked_at_micros: None,
        };
        session.validate()?;
        self.store
            .commit_login_success(LoginSuccess {
                session,
                session_created_event: security_event(
                    SecurityAction::SessionCreated,
                    &administrator,
                    context,
                )?,
                login_succeeded_event: security_event(
                    SecurityAction::LoginSucceeded,
                    &administrator,
                    context,
                )?,
            })
            .await
            .map_err(ServiceError::Store)?;
        self.clear_login_failures(source_key, account_key)?;
        Ok(AuthenticatedSession {
            administrator: sarmg_contracts::AdministratorSession::new(
                administrator.administrator_id.as_str(),
                administrator.username,
                &csrf_token,
            )
            .map_err(ServiceError::Contract)?,
            session_token,
        })
    }

    pub async fn restore_session(
        &self,
        session_token: &str,
        now_micros: u64,
    ) -> Result<AuthenticatedSession, ServiceError<Store::StoreError>> {
        if !sarmg_admin_auth::is_token_shape(session_token) {
            return Err(ServiceError::InvalidSession);
        }
        require_persistable_time(now_micros)?;
        let Some(found) = self
            .store
            .session_by_token_hash(sarmg_admin_auth::token_hash(session_token))
            .await
            .map_err(ServiceError::Store)?
        else {
            return Err(ServiceError::InvalidSession);
        };
        if found.session.status(&found.administrator, now_micros) != SessionStatus::Active {
            return Err(ServiceError::InvalidSession);
        }
        let csrf_token = sarmg_admin_auth::random_token()?;
        let idle_expires = now_micros
            .checked_add(SESSION_IDLE_MICROS)
            .ok_or(Error::InvalidTimestamp)?
            .min(found.session.absolute_expires_at_micros);
        self.store
            .rotate_session_csrf(
                &found.session.session_id,
                sarmg_admin_auth::token_hash(&csrf_token),
                now_micros,
                idle_expires,
            )
            .await
            .map_err(ServiceError::Store)?;
        Ok(AuthenticatedSession {
            administrator: sarmg_contracts::AdministratorSession::new(
                found.administrator.administrator_id.as_str(),
                found.administrator.username,
                &csrf_token,
            )
            .map_err(ServiceError::Contract)?,
            session_token: session_token.to_owned(),
        })
    }

    pub async fn authenticate_session(
        &self,
        session_token: &str,
        now_micros: u64,
    ) -> Result<AuthenticatedIdentity, ServiceError<Store::StoreError>> {
        if !sarmg_admin_auth::is_token_shape(session_token) {
            return Err(ServiceError::InvalidSession);
        }
        require_persistable_time(now_micros)?;
        let Some(found) = self
            .store
            .session_by_token_hash(sarmg_admin_auth::token_hash(session_token))
            .await
            .map_err(ServiceError::Store)?
        else {
            return Err(ServiceError::InvalidSession);
        };
        if found.session.status(&found.administrator, now_micros) != SessionStatus::Active {
            return Err(ServiceError::InvalidSession);
        }
        if found.session.should_write_last_seen(now_micros) {
            let idle_expires = now_micros
                .checked_add(SESSION_IDLE_MICROS)
                .ok_or(Error::InvalidTimestamp)?
                .min(found.session.absolute_expires_at_micros);
            self.store
                .rotate_session_csrf(
                    &found.session.session_id,
                    found.session.csrf_hash,
                    now_micros,
                    idle_expires,
                )
                .await
                .map_err(ServiceError::Store)?;
        }
        Ok(AuthenticatedIdentity {
            administrator_id: found.administrator.administrator_id,
            username: found.administrator.username,
            session_id: found.session.session_id,
            csrf_hash: found.session.csrf_hash,
        })
    }

    pub async fn logout(
        &self,
        session_token: &str,
        now_micros: u64,
        request_id: Option<String>,
    ) -> Result<(), ServiceError<Store::StoreError>> {
        if !sarmg_admin_auth::is_token_shape(session_token) {
            return Err(ServiceError::InvalidSession);
        }
        require_persistable_time(now_micros)?;
        let Some(found) = self
            .store
            .session_by_token_hash(sarmg_admin_auth::token_hash(session_token))
            .await
            .map_err(ServiceError::Store)?
        else {
            return Err(ServiceError::InvalidSession);
        };
        if found.session.status(&found.administrator, now_micros) != SessionStatus::Active {
            return Err(ServiceError::InvalidSession);
        }
        self.store
            .revoke_session(
                &found.session.session_id,
                now_micros,
                SecurityAuditEvent {
                    event_id: Identifier::new(sarmg_admin_auth::random_token()?)?,
                    action: SecurityAction::SessionRevoked,
                    outcome: AuditOutcome::Success,
                    actor_administrator_id: Some(found.administrator.administrator_id),
                    subject_digest: Some(sarmg_admin_auth::token_hash(
                        &found.administrator.username,
                    )),
                    request_id,
                    detail_json: "{}".into(),
                    occurred_at_micros: now_micros,
                },
            )
            .await
            .map_err(ServiceError::Store)
    }

    pub async fn require_csrf(
        &self,
        session_token: &str,
        csrf_header_values: &[Vec<u8>],
        now_micros: u64,
    ) -> Result<(), ServiceError<Store::StoreError>> {
        if !sarmg_admin_auth::is_token_shape(session_token) {
            return Err(ServiceError::InvalidSession);
        }
        let Some(found) = self
            .store
            .session_by_token_hash(sarmg_admin_auth::token_hash(session_token))
            .await
            .map_err(ServiceError::Store)?
        else {
            return Err(ServiceError::InvalidSession);
        };
        if found.session.status(&found.administrator, now_micros) != SessionStatus::Active {
            return Err(ServiceError::InvalidSession);
        }
        sarmg_admin_auth::require_csrf_token_matches_hash(
            csrf_header_values,
            &found.session.csrf_hash,
        )
        .map_err(ServiceError::Authentication)
    }

    fn require_login_capacity(
        &self,
        source: [u8; 32],
        account: [u8; 32],
        now: u64,
    ) -> Result<(), Error> {
        let mut failures = self
            .failures
            .lock()
            .map_err(|_| Error::LoginLimiterPoisoned)?;
        prune_failures(&mut failures, now);
        if failures
            .source
            .get(&source)
            .is_some_and(|values| values.len() >= FAILURES_PER_SOURCE)
            || failures
                .account
                .get(&account)
                .is_some_and(|values| values.len() >= FAILURES_PER_ACCOUNT)
            || (!failures.source.contains_key(&source)
                && failures.source.len() >= MAX_LOGIN_TRACKED_KEYS)
            || (!failures.account.contains_key(&account)
                && failures.account.len() >= MAX_LOGIN_TRACKED_KEYS)
        {
            Err(Error::LoginRateLimited)
        } else {
            Ok(())
        }
    }

    fn record_login_failure(
        &self,
        source: [u8; 32],
        account: [u8; 32],
        now: u64,
    ) -> Result<(), Error> {
        let mut failures = self
            .failures
            .lock()
            .map_err(|_| Error::LoginLimiterPoisoned)?;
        prune_failures(&mut failures, now);
        failures.source.entry(source).or_default().push_back(now);
        failures.account.entry(account).or_default().push_back(now);
        Ok(())
    }

    fn clear_login_failures(&self, source: [u8; 32], account: [u8; 32]) -> Result<(), Error> {
        let mut failures = self
            .failures
            .lock()
            .map_err(|_| Error::LoginLimiterPoisoned)?;
        failures.source.remove(&source);
        failures.account.remove(&account);
        Ok(())
    }
}

const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$MDEyMzQ1Njc4OWFiY2RlZg$5ZQ3RjSgS3eazD2qv8ZO2Cm50ONQ2RplT/MMLbScSMU";

fn prune_failures(failures: &mut LoginFailures, now: u64) {
    let threshold = now.saturating_sub(LOGIN_WINDOW_MICROS);
    for entries in [&mut failures.source, &mut failures.account] {
        entries.retain(|_, times| {
            while times.front().is_some_and(|value| *value <= threshold) {
                times.pop_front();
            }
            !times.is_empty()
        });
    }
}

fn security_event(
    action: SecurityAction,
    administrator: &AdministratorRecord,
    context: &LoginContext,
) -> Result<SecurityAuditEvent, Error> {
    Ok(SecurityAuditEvent {
        event_id: Identifier::new(sarmg_admin_auth::random_token()?)?,
        action,
        outcome: AuditOutcome::Success,
        actor_administrator_id: Some(administrator.administrator_id.clone()),
        subject_digest: Some(sarmg_admin_auth::token_hash(&administrator.username)),
        request_id: context.request_id.clone(),
        detail_json: "{}".into(),
        occurred_at_micros: context.now_micros,
    })
}

fn maintenance_event(
    action: SecurityAction,
    administrator: &AdministratorRecord,
    now_micros: u64,
) -> Result<SecurityAuditEvent, Error> {
    Ok(SecurityAuditEvent {
        event_id: Identifier::new(sarmg_admin_auth::random_token()?)?,
        action,
        outcome: AuditOutcome::Success,
        actor_administrator_id: None,
        subject_digest: Some(sarmg_admin_auth::token_hash(&administrator.username)),
        request_id: None,
        detail_json: "{}".into(),
        occurred_at_micros: now_micros,
    })
}

#[derive(Debug, Error)]
pub enum ServiceError<StoreError>
where
    StoreError: std::error::Error + Send + Sync + 'static,
{
    #[error("administrator was not found")]
    AdministratorNotFound,
    #[error("administrator credentials are invalid")]
    InvalidCredentials,
    #[error("administrator login is temporarily rate limited")]
    LoginRateLimited,
    #[error("password verification capacity is temporarily exhausted")]
    AuthenticationBusy,
    #[error("password verification worker exited unexpectedly")]
    AuthenticationWorkerFailed,
    #[error("administrator session is invalid or expired")]
    InvalidSession,
    #[error(transparent)]
    Policy(#[from] Error),
    #[error("administrator store failed: {0}")]
    Store(StoreError),
    #[error("administrator response contract failed: {0}")]
    Contract(sarmg_contracts::ValidationError),
    #[error("authentication primitive failed: {0}")]
    Authentication(#[from] sarmg_admin_auth::Error),
}

pub fn session_cookie_name(
    product_id: &str,
    mode: AdministratorOriginMode,
) -> Result<String, Error> {
    require_product_id(product_id)?;
    let prefix = match mode {
        AdministratorOriginMode::ProductionHttps => "__Host-sarmg-",
        AdministratorOriginMode::LoopbackDevelopmentHttp => "sarmg-",
    };
    Ok(format!("{prefix}{product_id}-session"))
}

pub fn session_set_cookie(
    product_id: &str,
    mode: AdministratorOriginMode,
    token: &str,
) -> Result<String, Error> {
    if !sarmg_admin_auth::is_token_shape(token) {
        return Err(Error::InvalidSessionToken);
    }
    let name = session_cookie_name(product_id, mode)?;
    let secure = match mode {
        AdministratorOriginMode::ProductionHttps => "; Secure",
        AdministratorOriginMode::LoopbackDevelopmentHttp => "",
    };
    Ok(format!(
        "{name}={token}; Path=/; HttpOnly{secure}; SameSite=Strict"
    ))
}

pub fn session_clear_cookie(
    product_id: &str,
    mode: AdministratorOriginMode,
) -> Result<String, Error> {
    let name = session_cookie_name(product_id, mode)?;
    let secure = match mode {
        AdministratorOriginMode::ProductionHttps => "; Secure",
        AdministratorOriginMode::LoopbackDevelopmentHttp => "",
    };
    Ok(format!(
        "{name}=; Path=/; HttpOnly{secure}; SameSite=Strict; Max-Age=0"
    ))
}

fn require_product_id(value: &str) -> Result<(), Error> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > 63
        || !bytes[0].is_ascii_lowercase()
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
    {
        Err(Error::InvalidProductId)
    } else {
        Ok(())
    }
}

fn require_persistable_time(value: u64) -> Result<(), Error> {
    if value <= i64::MAX as u64 {
        Ok(())
    } else {
        Err(Error::InvalidTimestamp)
    }
}

#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum Error {
    #[error("identifier must be 1 to 64 visible non-whitespace ASCII bytes")]
    InvalidIdentifier,
    #[error("product ID is not canonical")]
    InvalidProductId,
    #[error("session token is not one canonical Foundation token")]
    InvalidSessionToken,
    #[error("timestamp does not fit the persisted signed 64-bit representation")]
    InvalidTimestamp,
    #[error("administrator record violates current policy")]
    InvalidAdministratorRecord,
    #[error("session record violates current policy")]
    InvalidSessionRecord,
    #[error("security audit event violates current policy")]
    InvalidAuditEvent,
    #[error("login source is invalid")]
    InvalidLoginSource,
    #[error("request ID is invalid")]
    InvalidRequestId,
    #[error("administrator login is rate limited")]
    LoginRateLimited,
    #[error("login limiter state lock was poisoned")]
    LoginLimiterPoisoned,
    #[error(transparent)]
    Authentication(#[from] sarmg_admin_auth::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_and_cookie_contract_are_fixed() {
        let policy = AdministratorPolicyV1;
        assert_eq!(policy.session_idle_micros(), 1_800_000_000);
        assert_eq!(policy.sessions_global(), 1_024);
        let token = "A".repeat(43);
        assert_eq!(
            session_set_cookie(
                "sunshine-manager",
                AdministratorOriginMode::ProductionHttps,
                &token
            )
            .unwrap(),
            format!(
                "__Host-sarmg-sunshine-manager-session={token}; Path=/; HttpOnly; Secure; SameSite=Strict"
            )
        );
        assert!(
            !session_clear_cookie("dufs-ram", AdministratorOriginMode::LoopbackDevelopmentHttp)
                .unwrap()
                .contains("Secure")
        );
    }
}
