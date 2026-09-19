use super::*;

pub const ADMINISTRATORS_PATH: &str = "/api/v2/platform/administrators";

pub const ADMIN_ACCOUNT_PATH: &str = "/api/v2/platform/administrators/self";

/// Self-service always verifies the current password; blank new_password keeps it.
/// No account identifier is accepted from the caller.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdministratorAccountRequest {
    #[serde(deserialize_with = "deserialize_administrator_username_candidate")]
    pub username: String,
    #[serde(deserialize_with = "deserialize_secret_text")]
    pub current_password: String,
    #[serde(default)]
    pub new_password: Option<String>,
}
