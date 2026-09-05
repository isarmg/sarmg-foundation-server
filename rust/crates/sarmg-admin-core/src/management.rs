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
    #[error("administrator password work is unavailable")]
    Busy,
    #[error("administrator storage failed: {0}")]
    Store(StoreError),
}

impl<Store: AdministratorStore + 'static> AdministratorService<Store> {
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
        let username = sarmg_admin_auth::normalize_administrator_username(username)
            .map_err(|_| ManagementError::InvalidInput)?;
        let password_hash = self.management_password_hash(password).await?;
        let identifier = sarmg_admin_auth::random_token().map_err(|_| ManagementError::Busy)?;
        let record = AdministratorRecord {
            administrator_id: Identifier::new(identifier)
                .map_err(|_| ManagementError::InvalidInput)?,
            username,
            password_hash,
            active: true,
            session_version: 1,
            created_at_micros: context.now_micros,
            updated_at_micros: context.now_micros,
            last_login_at_micros: None,
        };
        self.store
            .manage_administrator(context, AdministratorMutation::Create(record))
            .await
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
