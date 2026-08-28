//! Stable, framework-neutral interfaces exposed by the Core Platform to plugins.

use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fs::File,
    future::Future,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    pin::Pin,
};

use sarmg_platform_core::{
    Execution, HttpMethod, MANIFEST_VERSION, PLATFORM_API_VERSION, PLUGIN_API_VERSION,
    PluginManifest, UNION_MODULE_AUDIENCE_ENV, UNION_MODULE_PREFIX_ENV, UNION_MODULE_PROTOCOL_ENV,
    UNION_MODULE_TOKEN_ENV, UNION_PLUGIN_BIND_ENV, UNION_PLUGIN_CONFIG_ENV, UNION_PLUGIN_ID_ENV,
    UNION_PLUGIN_PACKAGE_ROOT_ENV, UNION_PLUGIN_PORT_ENV, UNION_PLUGIN_VERSION_ENV,
};
use sarmg_platform_events::EventPublisher;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type PlatformFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlatformError {
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("platform capability is unavailable: {0}")]
    Unavailable(String),
    #[error("invalid plugin request: {0}")]
    Invalid(String),
    #[error("platform operation failed: {0}")]
    Operation(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Actor {
    pub subject: String,
    pub permissions: BTreeSet<String>,
    pub correlation_id: String,
}

pub trait ConfigurationApi: Send + Sync {
    fn get<'a>(
        &'a self,
        key: &'a str,
    ) -> PlatformFuture<'a, Result<Option<serde_json::Value>, PlatformError>>;
}

pub trait AuthorizationApi: Send + Sync {
    fn authorize(&self, actor: &Actor, permission: &str) -> Result<(), PlatformError>;
}

pub trait AuditApi: Send + Sync {
    fn record<'a>(
        &'a self,
        action: &'a str,
        actor: &'a Actor,
        fields: serde_json::Value,
    ) -> PlatformFuture<'a, Result<(), PlatformError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

pub trait LogApi: Send + Sync {
    fn write(
        &self,
        level: LogLevel,
        message: &str,
        fields: serde_json::Value,
    ) -> Result<(), PlatformError>;
}

pub trait TaskApi: Send + Sync {
    fn enqueue<'a>(
        &'a self,
        task: &'a str,
        payload: serde_json::Value,
    ) -> PlatformFuture<'a, Result<String, PlatformError>>;
}

pub trait NotificationApi: Send + Sync {
    fn notify<'a>(
        &'a self,
        channel: &'a str,
        message: &'a str,
    ) -> PlatformFuture<'a, Result<(), PlatformError>>;
}

pub trait ServiceDiscoveryApi: Send + Sync {
    fn resolve<'a>(&'a self, service: &'a str)
    -> PlatformFuture<'a, Result<String, PlatformError>>;
}

pub trait PlatformContext: Send + Sync {
    fn plugin_id(&self) -> &str;
    fn configuration(&self) -> &dyn ConfigurationApi;
    fn authorization(&self) -> &dyn AuthorizationApi;
    fn audit(&self) -> &dyn AuditApi;
    fn logs(&self) -> &dyn LogApi;
    fn tasks(&self) -> &dyn TaskApi;
    fn notifications(&self) -> &dyn NotificationApi;
    fn services(&self) -> &dyn ServiceDiscoveryApi;
    fn events(&self) -> &dyn EventPublisher;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ready,
    Degraded,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HealthReport {
    pub status: HealthStatus,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProcessHttpRequest {
    /// Manifest route id selected by Core after matching method and canonical path.
    pub route_id: String,
    pub method: HttpMethod,
    pub path: String,
    pub query: Option<String>,
    /// Lower-case, hop-by-hop-filtered headers. Authentication secrets are not forwarded.
    pub headers: BTreeMap<String, Vec<String>>,
    /// Bounded body buffered by Core. Large/streaming workloads belong in a process or service.
    pub body: Vec<u8>,
    pub actor: Option<Actor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InProcessHttpResponse {
    pub status: u16,
    pub headers: BTreeMap<String, Vec<String>>,
    pub body: Vec<u8>,
}

pub trait InProcessHttpHandler: Send + Sync {
    fn handle_http<'a>(
        &'a self,
        request: InProcessHttpRequest,
    ) -> PlatformFuture<'a, Result<InProcessHttpResponse, PlatformError>>;
}

/// In-process modules implement this trait. Other execution modes use equivalent wire contracts.
pub trait InProcessPlugin: InProcessHttpHandler {
    fn manifest(&self) -> &PluginManifest;
    fn start<'a>(
        &'a self,
        platform: &'a dyn PlatformContext,
    ) -> PlatformFuture<'a, Result<(), PlatformError>>;
    fn stop<'a>(&'a self) -> PlatformFuture<'a, Result<(), PlatformError>>;
    fn health<'a>(&'a self) -> PlatformFuture<'a, HealthReport>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginHandshake {
    pub manifest_version: u32,
    pub plugin_id: String,
    pub plugin_version: String,
    pub platform_api_version: String,
    pub plugin_api_version: String,
}

impl PluginHandshake {
    pub fn for_manifest(manifest: &PluginManifest) -> Self {
        Self {
            manifest_version: MANIFEST_VERSION,
            plugin_id: manifest.id.clone(),
            plugin_version: manifest.version.clone(),
            platform_api_version: PLATFORM_API_VERSION.into(),
            plugin_api_version: PLUGIN_API_VERSION.into(),
        }
    }
}

pub const MAX_CONFIGURATION_BYTES: u64 = 1024 * 1024;

/// Validated process-side runtime context. No configuration value is accepted on a command line.
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub plugin_id: String,
    pub plugin_version: String,
    pub bind: SocketAddr,
    pub package_root: PathBuf,
    pub configuration_path: PathBuf,
    pub gateway: GatewayEnvironment,
}

#[derive(Clone)]
pub struct GatewayEnvironment {
    pub protocol: String,
    pub audience: String,
    token: String,
    pub prefix: String,
}

impl std::fmt::Debug for GatewayEnvironment {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("GatewayEnvironment")
            .field("protocol", &self.protocol)
            .field("audience", &self.audience)
            .field("token", &"[REDACTED]")
            .field("prefix", &self.prefix)
            .finish()
    }
}

impl GatewayEnvironment {
    /// Token access is explicit so debug output never exposes it.
    pub fn token(&self) -> &str {
        &self.token
    }
}

#[derive(Debug, Error)]
pub enum ProcessContextError {
    #[error("required runtime environment variable is missing: {0}")]
    Missing(&'static str),
    #[error("runtime environment variable is invalid: {0}")]
    Invalid(&'static str),
    #[error("runtime plugin identity does not match manifest")]
    IdentityMismatch,
    #[error("runtime path must be absolute, regular and not a symbolic link: {0}")]
    UnsafePath(PathBuf),
    #[error("configuration exceeds {MAX_CONFIGURATION_BYTES} bytes")]
    ConfigurationTooLarge,
    #[error("configuration binding is missing or is not scalar: {0}")]
    InvalidBinding(String),
    #[error("configuration I/O failed")]
    Io(#[source] std::io::Error),
    #[error("configuration JSON is invalid")]
    Json(#[source] serde_json::Error),
}

impl ProcessContext {
    /// Read and validate the standard runtime environment against an already validated manifest.
    pub fn from_env(manifest: &PluginManifest) -> Result<Self, ProcessContextError> {
        if !matches!(&manifest.execution, Execution::Process { .. }) {
            return Err(ProcessContextError::Invalid("execution.mode"));
        }
        let plugin_id = required_env(UNION_PLUGIN_ID_ENV)?;
        let plugin_version = required_env(UNION_PLUGIN_VERSION_ENV)?;
        if plugin_id != manifest.id || plugin_version != manifest.version {
            return Err(ProcessContextError::IdentityMismatch);
        }
        let bind: SocketAddr = required_env(UNION_PLUGIN_BIND_ENV)?
            .parse()
            .map_err(|_| ProcessContextError::Invalid(UNION_PLUGIN_BIND_ENV))?;
        if !bind.ip().is_loopback() {
            return Err(ProcessContextError::Invalid(UNION_PLUGIN_BIND_ENV));
        }
        let port: u16 = required_env(UNION_PLUGIN_PORT_ENV)?
            .parse()
            .map_err(|_| ProcessContextError::Invalid(UNION_PLUGIN_PORT_ENV))?;
        if port == 0 || bind.port() != port {
            return Err(ProcessContextError::Invalid(UNION_PLUGIN_PORT_ENV));
        }

        let package_root = PathBuf::from(required_env(UNION_PLUGIN_PACKAGE_ROOT_ENV)?);
        validate_directory(&package_root)?;
        let configuration_path = PathBuf::from(required_env(UNION_PLUGIN_CONFIG_ENV)?);
        validate_regular_file(&configuration_path)?;

        let protocol = required_env(UNION_MODULE_PROTOCOL_ENV)?;
        let audience = required_env(UNION_MODULE_AUDIENCE_ENV)?;
        let token = required_env(UNION_MODULE_TOKEN_ENV)?;
        let prefix = required_env(UNION_MODULE_PREFIX_ENV)?;
        if protocol != "gateway-v1"
            || audience != manifest.id
            || prefix != format!("/api/modules/{}", manifest.id)
            || token.len() != 64
            || !token
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(ProcessContextError::Invalid(UNION_MODULE_PROTOCOL_ENV));
        }

        Ok(Self {
            plugin_id,
            plugin_version,
            bind,
            package_root,
            configuration_path,
            gateway: GatewayEnvironment {
                protocol,
                audience,
                token,
                prefix,
            },
        })
    }

    pub fn load_configuration<T: serde::de::DeserializeOwned>(
        &self,
    ) -> Result<T, ProcessContextError> {
        serde_json::from_slice(&read_limited_regular_file(&self.configuration_path)?)
            .map_err(ProcessContextError::Json)
    }

    pub fn load_configuration_value(&self) -> Result<serde_json::Value, ProcessContextError> {
        self.load_configuration()
    }
}

/// Resolve legacy environment aliases from already schema-validated configuration. Values are
/// passed directly to `Command::env`; no shell or string interpolation is involved.
pub fn resolve_environment_bindings(
    manifest: &PluginManifest,
    configuration: &serde_json::Value,
) -> Result<BTreeMap<String, String>, ProcessContextError> {
    let Execution::Process { environment, .. } = &manifest.execution else {
        return Ok(BTreeMap::new());
    };
    environment
        .iter()
        .filter_map(|binding| {
            let value = configuration.pointer(&binding.config_pointer)?;
            Some(
                scalar_to_string(value)
                    .map(|value| (binding.name.clone(), value))
                    .ok_or_else(|| {
                        ProcessContextError::InvalidBinding(binding.config_pointer.clone())
                    }),
            )
        })
        .collect()
}

fn scalar_to_string(value: &serde_json::Value) -> Option<String> {
    let rendered = match value {
        serde_json::Value::String(value) => Some(value.clone()),
        serde_json::Value::Bool(value) => Some(value.to_string()),
        serde_json::Value::Number(value) => Some(value.to_string()),
        serde_json::Value::Null | serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            None
        }
    }?;
    (rendered.len() <= 65_536 && !rendered.contains('\0')).then_some(rendered)
}

fn required_env(name: &'static str) -> Result<String, ProcessContextError> {
    env::var(name).map_err(|_| ProcessContextError::Missing(name))
}

fn validate_directory(path: &Path) -> Result<(), ProcessContextError> {
    if !path.is_absolute() {
        return Err(ProcessContextError::UnsafePath(path.to_owned()));
    }
    let metadata = std::fs::symlink_metadata(path).map_err(ProcessContextError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(ProcessContextError::UnsafePath(path.to_owned()));
    }
    Ok(())
}

fn validate_regular_file(path: &Path) -> Result<(), ProcessContextError> {
    if !path.is_absolute() {
        return Err(ProcessContextError::UnsafePath(path.to_owned()));
    }
    let metadata = std::fs::symlink_metadata(path).map_err(ProcessContextError::Io)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ProcessContextError::UnsafePath(path.to_owned()));
    }
    if metadata.len() > MAX_CONFIGURATION_BYTES {
        return Err(ProcessContextError::ConfigurationTooLarge);
    }
    Ok(())
}

fn read_limited_regular_file(path: &Path) -> Result<Vec<u8>, ProcessContextError> {
    validate_regular_file(path)?;
    let file = File::open(path).map_err(ProcessContextError::Io)?;
    let opened = file.metadata().map_err(ProcessContextError::Io)?;
    if !opened.is_file() || opened.len() > MAX_CONFIGURATION_BYTES {
        return Err(ProcessContextError::ConfigurationTooLarge);
    }
    let mut bytes = Vec::with_capacity(opened.len() as usize);
    file.take(MAX_CONFIGURATION_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(ProcessContextError::Io)?;
    if bytes.len() as u64 > MAX_CONFIGURATION_BYTES {
        return Err(ProcessContextError::ConfigurationTooLarge);
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sarmg_platform_core::PluginManifest;

    const PROCESS_FIXTURE: &str = include_str!("../../../tests/fixtures/process-module.json");

    fn context(path: PathBuf, root: PathBuf) -> ProcessContext {
        ProcessContext {
            plugin_id: "fixture-module".into(),
            plugin_version: "1.2.3".into(),
            bind: "127.0.0.1:18102".parse().unwrap(),
            package_root: root,
            configuration_path: path,
            gateway: GatewayEnvironment {
                protocol: "gateway-v1".into(),
                audience: "fixture-module".into(),
                token: "0".repeat(64),
                prefix: "/api/modules/fixture-module".into(),
            },
        }
    }

    #[test]
    fn regular_bounded_configuration_is_deserialized() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        std::fs::write(&path, br#"{"enabled":true}"#).unwrap();
        let value: serde_json::Value = context(path, directory.path().to_owned())
            .load_configuration()
            .unwrap();
        assert_eq!(value["enabled"], true);
    }

    #[cfg(unix)]
    #[test]
    fn symbolic_link_configuration_is_rejected() {
        use std::os::unix::fs::symlink;

        let directory = tempfile::tempdir().unwrap();
        let target = directory.path().join("target.json");
        let link = directory.path().join("config.json");
        std::fs::write(&target, b"{}").unwrap();
        symlink(&target, &link).unwrap();
        assert!(matches!(
            context(link, directory.path().to_owned()).load_configuration_value(),
            Err(ProcessContextError::UnsafePath(_))
        ));
    }

    #[test]
    fn oversized_configuration_is_rejected_before_json_parsing() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let file = File::create(&path).unwrap();
        file.set_len(MAX_CONFIGURATION_BYTES + 1).unwrap();
        assert!(matches!(
            context(path, directory.path().to_owned()).load_configuration_value(),
            Err(ProcessContextError::ConfigurationTooLarge)
        ));
    }

    #[test]
    fn configuration_environment_bindings_are_scalar_and_non_shell() {
        let manifest = PluginManifest::parse_json(PROCESS_FIXTURE).unwrap();
        let values = serde_json::json!({
            "database_url": "postgresql://localhost/fixture",
            "data_dir": "/srv/fixture",
            "max_batch": 1024,
            "require_tls": true
        });
        let environment = resolve_environment_bindings(&manifest, &values).unwrap();
        assert_eq!(environment["FIXTURE_MAX_BATCH"], "1024");
        assert_eq!(environment["FIXTURE_REQUIRE_TLS"], "true");
        assert_eq!(environment["FIXTURE_DATA_DIR"], "/srv/fixture");
        assert!(!environment.contains_key("FIXTURE_OPTIONAL_TOKEN"));
    }

    #[test]
    fn handshake_uses_api_versions_not_crate_version() {
        let manifest = PluginManifest::parse_json(PROCESS_FIXTURE).unwrap();
        let handshake = PluginHandshake::for_manifest(&manifest);
        assert_eq!(handshake.platform_api_version, "1.0.0");
        assert_eq!(handshake.plugin_api_version, "1.0.0");
        assert_eq!(handshake.plugin_id, "fixture-module");
        assert_eq!(handshake.plugin_version, "1.2.3");
    }

    #[test]
    fn gateway_debug_output_redacts_process_token() {
        let gateway = GatewayEnvironment {
            protocol: "gateway-v1".into(),
            audience: "fixture-module".into(),
            token: "a".repeat(64),
            prefix: "/api/modules/fixture-module".into(),
        };
        let rendered = format!("{gateway:?}");
        assert!(rendered.contains("[REDACTED]"));
        assert!(!rendered.contains(&"a".repeat(64)));
    }
}
