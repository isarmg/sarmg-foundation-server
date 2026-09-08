use super::*;

/// The authorization snapshot must be revalidated by the store in the same
/// transaction as the mutation, after any password hashing work has finished.
#[derive(Clone, Debug)]
pub struct AdministratorManagementContext {
    pub identity: AuthenticatedIdentity,
    pub request_id: Option<String>,
    pub now_micros: u64,
}

impl AdministratorManagementContext {
    pub fn validate(&self) -> Result<(), Error> {
        require_persistable_time(self.now_micros)?;
        if let Some(id) = &self.request_id {
            sarmg_contracts::RequestId::new(id.clone()).map_err(|_| Error::InvalidRequestId)?;
        }
        Ok(())
    }
}

/// Hashes and internal records never implement Serialize for a public response.
pub enum AdministratorMutation {
    Create(AdministratorRecord),
    UpdateOwnAccount {
        username: String,
        expected_password_hash: String,
        password_hash: Option<String>,
    },
    ChangePassword {
        administrator_id: Identifier,
        password_hash: String,
    },
    Disable {
        administrator_id: Identifier,
    },
}

pub use sarmg_contracts::AdministratorSummary;

impl From<AdministratorRecord> for AdministratorSummary {
    fn from(value: AdministratorRecord) -> Self {
        Self {
            administrator_id: value.administrator_id.to_string(),
            username: value.username,
            active: value.active,
            created_at_micros: value.created_at_micros,
            updated_at_micros: value.updated_at_micros,
            last_login_at_micros: value.last_login_at_micros,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ManagementError<StoreError: std::error::Error + Send + Sync + 'static> {
    #[error("administrator management is not supported by this profile")]
    Unsupported,
    #[error("administrator authorization is no longer valid")]
    Unauthorized,
    #[error("administrator does not exist")]
    NotFound,
    #[error("administrator already exists or cannot be changed in this state")]
    Conflict,
    #[error("the final active administrator cannot be disabled")]
    LastAdministrator,
    #[error("administrator input is invalid")]
    InvalidInput,
    #[error("current administrator password is incorrect")]
    InvalidCurrentPassword,
    #[error("administrator password work is unavailable")]
    Busy,
    #[error("administrator storage failed: {0}")]
    Store(StoreError),
}

impl<Store: AdministratorStore + 'static> AdministratorService<Store> {
    /// Verify the current credential before atomically updating the session's own
    /// account. The store repeats session/CSRF checks under its mutation lock.
    pub async fn update_own_account(
        &self,
        context: &AdministratorManagementContext,
        username: &str,
        current_password: &str,
        new_password: Option<&str>,
    ) -> Result<(), ManagementError<Store::StoreError>> {
        if !self.store.supports_account_updates() {
            return Err(ManagementError::Unsupported);
        }
        context
            .validate()
            .map_err(|_| ManagementError::InvalidInput)?;
        let username = sarmg_admin_auth::normalize_administrator_username(username)
            .map_err(|_| ManagementError::InvalidInput)?;
        if current_password.len() > 1024 {
            return Err(ManagementError::InvalidInput);
        }
        let record = self
            .store
            .administrator_by_username(&context.identity.username)
            .await
            .map_err(ManagementError::Store)?
            .ok_or(ManagementError::Unauthorized)?;
        if !record.active || record.administrator_id != context.identity.administrator_id {
            return Err(ManagementError::Unauthorized);
        }
        let account_key = sarmg_admin_auth::token_hash(&record.username);
        let source_key = sarmg_admin_auth::token_hash(context.identity.session_id.as_str());
        self.require_login_capacity(source_key, account_key, context.now_micros)
            .map_err(|_| ManagementError::Busy)?;
        let permit = tokio::time::timeout(
            Duration::from_micros(ARGON2_ACQUIRE_TIMEOUT_MICROS),
            self.argon2_slots.clone().acquire_owned(),
        )
        .await
        .map_err(|_| ManagementError::Busy)?
        .map_err(|_| ManagementError::Busy)?;
        let encoded = record.password_hash.clone();
        let candidate = current_password.to_owned();
        let matches = tokio::task::spawn_blocking(move || {
            let matches = sarmg_admin_auth::verify_password(&candidate, &encoded);
            drop(permit);
            matches
        })
        .await
        .map_err(|_| ManagementError::Busy)?;
        if !matches {
            self.record_login_failure(source_key, account_key, context.now_micros)
                .map_err(|_| ManagementError::Busy)?;
            return Err(ManagementError::InvalidCurrentPassword);
        }
        let password_hash = match new_password.filter(|value| !value.is_empty()) {
            Some(value) => Some(self.management_password_hash(value).await?),
            None => None,
        };
        self.store
            .manage_administrator(
                context,
                AdministratorMutation::UpdateOwnAccount {
                    username,
                    expected_password_hash: record.password_hash,
                    password_hash,
                },
            )
            .await
    }

    pub async fn list_administrators(
        &self,
        limit: u32,
        offset: u64,
    ) -> Result<Vec<AdministratorSummary>, ManagementError<Store::StoreError>> {
        if !self.store.supports_management() {
            return Err(ManagementError::Unsupported);
        }
        if !(1..=100).contains(&limit) || offset > i64::MAX as u64 {
            return Err(ManagementError::InvalidInput);
        }
        self.store
            .list_administrators(limit, offset)
            .await
            .map(|records| {
                records
                    .into_iter()
                    .map(AdministratorSummary::from)
                    .collect()
            })
            .map_err(ManagementError::Store)
    }

    pub async fn create_administrator(
        &self,
        context: &AdministratorManagementContext,
        username: &str,
        password: &str,
    ) -> Result<(), ManagementError<Store::StoreError>> {
        self.require_management(context)?;
        let _ = (username, password);
        Err(ManagementError::Conflict)
    }

    pub async fn set_administrator_password(
        &self,
        context: &AdministratorManagementContext,
        administrator_id: &str,
        password: &str,
    ) -> Result<(), ManagementError<Store::StoreError>> {
        self.require_management(context)?;
        let administrator_id =
            Identifier::new(administrator_id).map_err(|_| ManagementError::InvalidInput)?;
        let password_hash = self.management_password_hash(password).await?;
        self.store
            .manage_administrator(
                context,
                AdministratorMutation::ChangePassword {
                    administrator_id,
                    password_hash,
                },
            )
            .await
    }

    pub async fn disable_administrator(
        &self,
        context: &AdministratorManagementContext,
        administrator_id: &str,
    ) -> Result<(), ManagementError<Store::StoreError>> {
        self.require_management(context)?;
        let administrator_id =
            Identifier::new(administrator_id).map_err(|_| ManagementError::InvalidInput)?;
        self.store
            .manage_administrator(context, AdministratorMutation::Disable { administrator_id })
            .await
    }

    fn require_management(
        &self,
        context: &AdministratorManagementContext,
    ) -> Result<(), ManagementError<Store::StoreError>> {
        if !self.store.supports_management() {
            return Err(ManagementError::Unsupported);
        }
        context
            .validate()
            .map_err(|_| ManagementError::InvalidInput)
    }

    async fn management_password_hash(
        &self,
        password: &str,
    ) -> Result<String, ManagementError<Store::StoreError>> {
        self.hash_password(password)
            .await
            .map_err(|error| match error {
                ServiceError::AuthenticationBusy | ServiceError::AuthenticationWorkerFailed => {
                    ManagementError::Busy
                }
                _ => ManagementError::InvalidInput,
            })
    }
}
