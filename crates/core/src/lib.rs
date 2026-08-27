//! Framework-independent contracts for private Union process modules.

use std::{
    collections::{BTreeMap, BTreeSet},
    net::SocketAddr,
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Versioned descriptors shipped with the platform contract and embedded in Union at compile
/// time. A copied worker binary that has no matching descriptor remains unreachable.
pub mod manifests {
    pub const SUNSHINE: &str = include_str!("../../../modules/sunshine.json");
    pub const HOST_MONITORING: &str = include_str!("../../../modules/host-monitoring.json");
    pub const SENTINEL_MONITOR: &str = include_str!("../../../modules/sentinel-monitor.json");
    pub const PHOTO_BACKUP: &str = include_str!("../../../modules/photo-backup.json");
    pub const DUFS: &str = include_str!("../../../modules/dufs.json");

    pub const ALL: [&str; 5] = [
        SUNSHINE,
        HOST_MONITORING,
        SENTINEL_MONITOR,
        PHOTO_BACKUP,
        DUFS,
    ];
}

/// Every business module is selected at compile time and runs as a supervised private process.
/// There is deliberately no in-process or arbitrary external-service variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleExecution {
    PrivateProcess,
}

/// A browser-visible contribution is either a compile-time Union console view or a worker-owned
/// application below the module's fixed gateway prefix. This choice does not change the process
/// boundary: both variants still use a private worker for business APIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiContribution {
    Console { route: String },
    Gateway { entry_path: String },
}

/// Immutable process topology compiled into Union and the release manifest.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceBinding {
    /// Logical installed filename below `libexec/union/modules/`.
    pub binary: String,
    /// Fixed loopback socket. It is not a user-configurable upstream.
    pub bind: SocketAddr,
    /// Fixed browser/API prefix exposed by Union.
    pub gateway_prefix: String,
    pub liveness_path: String,
    pub readiness_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "profile", rename_all = "snake_case", deny_unknown_fields)]
pub enum DatabaseOwnership {
    /// A schema and runtime role used only by one module within the shared platform database.
    PostgresSchema {
        database_env: String,
        schema: String,
        role: String,
    },
    /// A module-owned PostgreSQL database and role in the common operational cluster.
    DedicatedPostgres { database_env: String, role: String },
    /// The deliberate Dufs exception: state shares the filesystem failure domain with its root.
    EmbeddedSqlite {
        state_directory: String,
        rationale: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModuleDescriptor {
    pub schema_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub execution: ModuleExecution,
    pub ui: UiContribution,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub service: ServiceBinding,
    pub database: DatabaseOwnership,
}

impl ModuleDescriptor {
    /// Relative same-origin entry exposed to browsers. No environment-derived public URL is
    /// involved; the result is a pure function of the embedded descriptor.
    pub fn browser_entry_path(&self) -> String {
        match &self.ui {
            UiContribution::Console { route } => route.clone(),
            UiContribution::Gateway { entry_path } if entry_path == "/" => {
                format!("{}/", self.service.gateway_prefix)
            }
            UiContribution::Gateway { entry_path } => {
                format!("{}{}", self.service.gateway_prefix, entry_path)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleHealthState {
    Starting,
    Probing,
    Available,
    Degraded,
    Backoff,
    Unconfigured,
    Stopped,
}

/// Runtime state returned by Union for a descriptor already selected at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInstance {
    #[serde(flatten)]
    pub descriptor: ModuleDescriptor,
    pub health: ModuleHealthState,
    pub health_message: String,
    pub pid: Option<u32>,
    pub restart_count: u64,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleCatalog {
    modules: Vec<ModuleDescriptor>,
    by_id: BTreeMap<String, usize>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CatalogError {
    #[error("module schema version must be 1 for {0}")]
    UnsupportedSchema(String),
    #[error("module id is invalid: {0}")]
    InvalidId(String),
    #[error("module display name must not be empty: {0}")]
    EmptyDisplayName(String),
    #[error("module description must not be empty: {0}")]
    EmptyDescription(String),
    #[error("module version must not be empty: {0}")]
    EmptyVersion(String),
    #[error("duplicate module id: {0}")]
    DuplicateId(String),
    #[error("module worker binary must equal its id: {module} uses {binary}")]
    InvalidBinary { module: String, binary: String },
    #[error("duplicate module worker binary: {0}")]
    DuplicateBinary(String),
    #[error("module binding must be a nonzero loopback socket for {module}: {bind}")]
    InvalidBinding { module: String, bind: SocketAddr },
    #[error("duplicate module binding: {0}")]
    DuplicateBinding(SocketAddr),
    #[error("module gateway prefix must be /modules/<id>: {module} uses {path}")]
    InvalidGatewayPrefix { module: String, path: String },
    #[error("duplicate module gateway prefix: {0}")]
    DuplicateGatewayPrefix(String),
    #[error("invalid module path for {module}: {path}")]
    InvalidPath { module: String, path: String },
    #[error("liveness and readiness paths must differ for {0}")]
    DuplicateHealthPath(String),
    #[error("invalid module capability for {module}: {capability}")]
    InvalidCapability { module: String, capability: String },
    #[error("duplicate module capability for {module}: {capability}")]
    DuplicateCapability { module: String, capability: String },
    #[error("database environment name is invalid for {module}: {name}")]
    InvalidDatabaseEnvironment { module: String, name: String },
    #[error("PostgreSQL identifier is invalid for {module}: {identifier}")]
    InvalidPostgresIdentifier { module: String, identifier: String },
    #[error("duplicate PostgreSQL role: {0}")]
    DuplicateDatabaseRole(String),
    #[error("duplicate PostgreSQL schema: {0}")]
    DuplicateDatabaseSchema(String),
    #[error("duplicate PostgreSQL environment name: {0}")]
    DuplicateDatabaseEnvironment(String),
    #[error("embedded SQLite state directory is invalid for {module}: {path}")]
    InvalidStateDirectory { module: String, path: String },
    #[error("embedded SQLite rationale must not be empty for {0}")]
    EmptySqliteRationale(String),
}

impl ModuleCatalog {
    pub fn new(modules: Vec<ModuleDescriptor>) -> Result<Self, CatalogError> {
        let mut ids = BTreeSet::new();
        let mut binaries = BTreeSet::new();
        let mut bindings = BTreeSet::new();
        let mut gateway_prefixes = BTreeSet::new();
        let mut database_roles = BTreeSet::new();
        let mut database_schemas = BTreeSet::new();
        let mut database_environments = BTreeSet::new();
        let mut by_id = BTreeMap::new();

        for (index, module) in modules.iter().enumerate() {
            validate_module(module)?;
            if !ids.insert(module.id.clone()) {
                return Err(CatalogError::DuplicateId(module.id.clone()));
            }
            if !binaries.insert(module.service.binary.clone()) {
                return Err(CatalogError::DuplicateBinary(module.service.binary.clone()));
            }
            if !bindings.insert(module.service.bind) {
                return Err(CatalogError::DuplicateBinding(module.service.bind));
            }
            if !gateway_prefixes.insert(module.service.gateway_prefix.clone()) {
                return Err(CatalogError::DuplicateGatewayPrefix(
                    module.service.gateway_prefix.clone(),
                ));
            }
            match &module.database {
                DatabaseOwnership::PostgresSchema {
                    database_env,
                    schema,
                    role,
                } => {
                    if !database_schemas.insert(schema.clone()) {
                        return Err(CatalogError::DuplicateDatabaseSchema(schema.clone()));
                    }
                    insert_postgres_identity(
                        database_env,
                        role,
                        &mut database_environments,
                        &mut database_roles,
                    )?;
                }
                DatabaseOwnership::DedicatedPostgres { database_env, role } => {
                    insert_postgres_identity(
                        database_env,
                        role,
                        &mut database_environments,
                        &mut database_roles,
                    )?
                }
                DatabaseOwnership::EmbeddedSqlite { .. } => {}
            }
            by_id.insert(module.id.clone(), index);
        }

        Ok(Self { modules, by_id })
    }

    pub fn modules(&self) -> &[ModuleDescriptor] {
        &self.modules
    }

    pub fn get(&self, id: &str) -> Option<&ModuleDescriptor> {
        self.by_id.get(id).map(|index| &self.modules[*index])
    }
}

fn insert_postgres_identity(
    environment: &str,
    role: &str,
    environments: &mut BTreeSet<String>,
    roles: &mut BTreeSet<String>,
) -> Result<(), CatalogError> {
    if !environments.insert(environment.to_string()) {
        return Err(CatalogError::DuplicateDatabaseEnvironment(
            environment.to_string(),
        ));
    }
    if !roles.insert(role.to_string()) {
        return Err(CatalogError::DuplicateDatabaseRole(role.to_string()));
    }
    Ok(())
}

fn validate_module(module: &ModuleDescriptor) -> Result<(), CatalogError> {
    if module.schema_version != 1 {
        return Err(CatalogError::UnsupportedSchema(module.id.clone()));
    }
    if !valid_module_id(&module.id) {
        return Err(CatalogError::InvalidId(module.id.clone()));
    }
    if module.display_name.trim().is_empty() || module.display_name.len() > 128 {
        return Err(CatalogError::EmptyDisplayName(module.id.clone()));
    }
    if module.description.trim().is_empty() || module.description.len() > 512 {
        return Err(CatalogError::EmptyDescription(module.id.clone()));
    }
    if module.version.trim().is_empty() || module.version.len() > 64 {
        return Err(CatalogError::EmptyVersion(module.id.clone()));
    }
    if module.service.binary != module.id {
        return Err(CatalogError::InvalidBinary {
            module: module.id.clone(),
            binary: module.service.binary.clone(),
        });
    }
    if !module.service.bind.ip().is_loopback() || module.service.bind.port() == 0 {
        return Err(CatalogError::InvalidBinding {
            module: module.id.clone(),
            bind: module.service.bind,
        });
    }
    let expected_prefix = format!("/modules/{}", module.id);
    if module.service.gateway_prefix != expected_prefix {
        return Err(CatalogError::InvalidGatewayPrefix {
            module: module.id.clone(),
            path: module.service.gateway_prefix.clone(),
        });
    }
    let ui_path = match &module.ui {
        UiContribution::Console { route } => route,
        UiContribution::Gateway { entry_path } => entry_path,
    };
    for path in [
        ui_path,
        &module.service.liveness_path,
        &module.service.readiness_path,
    ] {
        if !valid_absolute_path(path) {
            return Err(CatalogError::InvalidPath {
                module: module.id.clone(),
                path: path.clone(),
            });
        }
    }
    if module.service.liveness_path == module.service.readiness_path {
        return Err(CatalogError::DuplicateHealthPath(module.id.clone()));
    }
    let mut capabilities = BTreeSet::new();
    for capability in &module.capabilities {
        if !valid_capability(capability) {
            return Err(CatalogError::InvalidCapability {
                module: module.id.clone(),
                capability: capability.clone(),
            });
        }
        if !capabilities.insert(capability) {
            return Err(CatalogError::DuplicateCapability {
                module: module.id.clone(),
                capability: capability.clone(),
            });
        }
    }
    validate_database(module)
}

fn validate_database(module: &ModuleDescriptor) -> Result<(), CatalogError> {
    match &module.database {
        DatabaseOwnership::PostgresSchema {
            database_env,
            schema,
            role,
        } => {
            validate_database_environment(&module.id, database_env)?;
            validate_postgres_identifier(&module.id, schema)?;
            validate_postgres_identifier(&module.id, role)
        }
        DatabaseOwnership::DedicatedPostgres { database_env, role } => {
            validate_database_environment(&module.id, database_env)?;
            validate_postgres_identifier(&module.id, role)
        }
        DatabaseOwnership::EmbeddedSqlite {
            state_directory,
            rationale,
        } => {
            if !valid_relative_directory(state_directory) {
                return Err(CatalogError::InvalidStateDirectory {
                    module: module.id.clone(),
                    path: state_directory.clone(),
                });
            }
            if rationale.trim().is_empty() || rationale.len() > 512 {
                return Err(CatalogError::EmptySqliteRationale(module.id.clone()));
            }
            Ok(())
        }
    }
}

fn validate_database_environment(module: &str, name: &str) -> Result<(), CatalogError> {
    if name.is_empty()
        || name.len() > 128
        || !name.starts_with(|character: char| character.is_ascii_uppercase())
        || !name.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Err(CatalogError::InvalidDatabaseEnvironment {
            module: module.to_string(),
            name: name.to_string(),
        });
    }
    Ok(())
}

fn validate_postgres_identifier(module: &str, value: &str) -> Result<(), CatalogError> {
    if value.is_empty()
        || value.len() > 63
        || !value.starts_with(|character: char| character.is_ascii_lowercase())
        || !value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return Err(CatalogError::InvalidPostgresIdentifier {
            module: module.to_string(),
            identifier: value.to_string(),
        });
    }
    Ok(())
}

fn valid_module_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.ends_with(|character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        })
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
}

fn valid_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 512
        && !value.contains(['?', '#', '\\'])
        && !value.chars().any(char::is_control)
        && !value
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
}

fn valid_relative_directory(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains(['\\', ':'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && !matches!(segment, "." | ".."))
}

fn valid_capability(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.ends_with(|character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        })
        && value.chars().all(|character| {
            character.is_ascii_lowercase()
                || character.is_ascii_digit()
                || matches!(character, '.' | '-')
        })
        && !value.contains("..")
        && !value.contains("--")
        && !value.contains(".-")
        && !value.contains("-.")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped_modules() -> Vec<ModuleDescriptor> {
        manifests::ALL
            .into_iter()
            .map(|json| serde_json::from_str(json).unwrap())
            .collect()
    }

    #[test]
    fn shipped_manifests_are_the_exact_private_process_topology() {
        let catalog = ModuleCatalog::new(shipped_modules()).unwrap();
        let expected = [
            ("sentinel-monitor", "127.0.0.1:18101"),
            ("photo-backup", "127.0.0.1:18102"),
            ("dufs", "127.0.0.1:18103"),
            ("sunshine", "127.0.0.1:18104"),
            ("host-monitoring", "127.0.0.1:18105"),
        ];
        assert_eq!(catalog.modules().len(), expected.len());
        for (id, bind) in expected {
            let module = catalog.get(id).unwrap();
            assert_eq!(module.execution, ModuleExecution::PrivateProcess);
            assert_eq!(module.service.binary, id);
            assert_eq!(module.service.bind, bind.parse().unwrap());
            assert_eq!(module.service.gateway_prefix, format!("/modules/{id}"));
            match module.id.as_str() {
                "sunshine" | "host-monitoring" => {
                    assert!(matches!(module.ui, UiContribution::Console { .. }));
                    assert!(module.browser_entry_path().starts_with("/modules/"));
                }
                _ => {
                    assert!(matches!(module.ui, UiContribution::Gateway { .. }));
                    assert!(module.browser_entry_path().starts_with("/modules/"));
                }
            }
        }
    }

    #[test]
    fn shipped_database_ownership_matches_failure_domains() {
        let catalog = ModuleCatalog::new(shipped_modules()).unwrap();
        for id in ["sunshine", "host-monitoring"] {
            assert!(matches!(
                catalog.get(id).unwrap().database,
                DatabaseOwnership::PostgresSchema { .. }
            ));
        }
        for id in ["sentinel-monitor", "photo-backup"] {
            assert!(matches!(
                catalog.get(id).unwrap().database,
                DatabaseOwnership::DedicatedPostgres { .. }
            ));
        }
        assert!(matches!(
            catalog.get("dufs").unwrap().database,
            DatabaseOwnership::EmbeddedSqlite { .. }
        ));
    }

    #[test]
    fn json_schema_tracks_the_final_private_process_model() {
        let schema: serde_json::Value =
            serde_json::from_str(include_str!("../../../contracts/module-v1.schema.json")).unwrap();

        assert_eq!(
            schema["properties"]["execution"]["const"],
            "private_process"
        );
        assert_eq!(schema["$defs"]["ui"]["oneOf"].as_array().unwrap().len(), 2);

        let required_service_fields = schema["$defs"]["service"]["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap())
            .collect::<BTreeSet<_>>();
        assert_eq!(
            required_service_fields,
            BTreeSet::from([
                "binary",
                "bind",
                "gateway_prefix",
                "liveness_path",
                "readiness_path",
            ])
        );

        let database_profiles = [
            schema["$defs"]["postgresSchema"]["properties"]["profile"]["const"]
                .as_str()
                .unwrap(),
            schema["$defs"]["dedicatedPostgres"]["properties"]["profile"]["const"]
                .as_str()
                .unwrap(),
            schema["$defs"]["embeddedSqlite"]["properties"]["profile"]["const"]
                .as_str()
                .unwrap(),
        ];
        assert_eq!(
            database_profiles,
            ["postgres_schema", "dedicated_postgres", "embedded_sqlite"]
        );

        let serialized = schema.to_string();
        for removed_field in ["base_url_env", "in_process", "external_service"] {
            assert!(!serialized.contains(removed_field));
        }
    }

    #[test]
    fn duplicate_process_and_database_ownership_are_rejected() {
        let mut modules = shipped_modules();
        modules[1].service.bind = modules[0].service.bind;
        assert!(matches!(
            ModuleCatalog::new(modules),
            Err(CatalogError::DuplicateBinding(_))
        ));

        let mut modules = shipped_modules();
        let DatabaseOwnership::DedicatedPostgres { role, .. } = modules[2].database.clone() else {
            unreachable!()
        };
        let DatabaseOwnership::DedicatedPostgres {
            role: second_role, ..
        } = &mut modules[3].database
        else {
            unreachable!()
        };
        *second_role = role;
        assert!(matches!(
            ModuleCatalog::new(modules),
            Err(CatalogError::DuplicateDatabaseRole(_))
        ));
    }

    #[test]
    fn arbitrary_upstreams_and_unsafe_paths_are_not_representable() {
        let mut module = shipped_modules().remove(2);
        module.service.bind = "192.0.2.10:18101".parse().unwrap();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidBinding { .. })
        ));

        let mut module = shipped_modules().remove(0);
        module.service.gateway_prefix = "/proxy/anything".into();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidGatewayPrefix { .. })
        ));

        let mut module = shipped_modules().remove(0);
        module.service.liveness_path = "http://other-host/health".into();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidPath { .. })
        ));
    }

    #[test]
    fn binary_and_database_names_are_fail_closed() {
        let mut module = shipped_modules().remove(0);
        module.service.binary = "../sentinel".into();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidBinary { .. })
        ));

        let mut module = shipped_modules().remove(2);
        let DatabaseOwnership::DedicatedPostgres { database_env, .. } = &mut module.database else {
            unreachable!()
        };
        *database_env = "DATABASE_URL;COMMAND".into();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidDatabaseEnvironment { .. })
        ));
    }
}
