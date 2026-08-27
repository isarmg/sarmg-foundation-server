//! Framework-independent contracts for Sarmg platform modules.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Versioned module descriptors shipped with the platform contract.
///
/// Consumers use these constants instead of reaching into a sibling checkout, so an exact Git
/// dependency is sufficient to reproduce a Union build.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleExecution {
    InProcess,
    Service,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum UiContribution {
    Embedded { route: String },
    External { public_url_env: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServiceBinding {
    pub base_url_env: String,
    pub liveness_path: String,
    pub readiness_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "profile", rename_all = "snake_case")]
pub enum DatabaseOwnership {
    PlatformPostgresSchema { schema: String },
    DedicatedPostgres { database_env: String },
    EmbeddedSqlite { rationale: String },
    None,
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
    pub service: Option<ServiceBinding>,
    pub database: DatabaseOwnership,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModuleHealthState {
    Available,
    Degraded,
    Probing,
    Unconfigured,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleInstance {
    #[serde(flatten)]
    pub descriptor: ModuleDescriptor,
    pub configured: bool,
    pub health: ModuleHealthState,
    pub health_message: String,
    pub launch_url: Option<String>,
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
    #[error("module version must not be empty: {0}")]
    EmptyVersion(String),
    #[error("duplicate module id: {0}")]
    DuplicateId(String),
    #[error("duplicate embedded UI route: {0}")]
    DuplicateRoute(String),
    #[error("invalid module route for {module}: {route}")]
    InvalidRoute { module: String, route: String },
    #[error("service module is missing a service binding: {0}")]
    MissingServiceBinding(String),
    #[error("in-process module must not declare a service binding: {0}")]
    UnexpectedServiceBinding(String),
    #[error("service path is invalid for {module}: {path}")]
    InvalidServicePath { module: String, path: String },
    #[error("database schema is invalid for {module}: {schema}")]
    InvalidDatabaseSchema { module: String, schema: String },
}

impl ModuleCatalog {
    pub fn new(modules: Vec<ModuleDescriptor>) -> Result<Self, CatalogError> {
        let mut ids = BTreeSet::new();
        let mut routes = BTreeSet::new();
        let mut by_id = BTreeMap::new();

        for (index, module) in modules.iter().enumerate() {
            validate_module(module)?;
            if !ids.insert(module.id.clone()) {
                return Err(CatalogError::DuplicateId(module.id.clone()));
            }
            if let UiContribution::Embedded { route } = &module.ui
                && !routes.insert(route.clone())
            {
                return Err(CatalogError::DuplicateRoute(route.clone()));
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

fn validate_module(module: &ModuleDescriptor) -> Result<(), CatalogError> {
    if module.schema_version != 1 {
        return Err(CatalogError::UnsupportedSchema(module.id.clone()));
    }
    if !valid_identifier(&module.id, '-') {
        return Err(CatalogError::InvalidId(module.id.clone()));
    }
    if module.display_name.trim().is_empty() {
        return Err(CatalogError::EmptyDisplayName(module.id.clone()));
    }
    if module.version.trim().is_empty() {
        return Err(CatalogError::EmptyVersion(module.id.clone()));
    }
    match &module.ui {
        UiContribution::Embedded { route } => {
            if !valid_absolute_path(route) {
                return Err(CatalogError::InvalidRoute {
                    module: module.id.clone(),
                    route: route.clone(),
                });
            }
        }
        UiContribution::External { public_url_env } => {
            if public_url_env.trim().is_empty() {
                return Err(CatalogError::InvalidRoute {
                    module: module.id.clone(),
                    route: public_url_env.clone(),
                });
            }
        }
    }
    match (&module.execution, &module.service) {
        (ModuleExecution::Service, None) => {
            return Err(CatalogError::MissingServiceBinding(module.id.clone()));
        }
        (ModuleExecution::InProcess, Some(_)) => {
            return Err(CatalogError::UnexpectedServiceBinding(module.id.clone()));
        }
        _ => {}
    }
    if let Some(binding) = &module.service {
        for path in [
            Some(binding.liveness_path.as_str()),
            binding.readiness_path.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if !valid_absolute_path(path) {
                return Err(CatalogError::InvalidServicePath {
                    module: module.id.clone(),
                    path: path.to_string(),
                });
            }
        }
    }
    if let DatabaseOwnership::PlatformPostgresSchema { schema } = &module.database
        && !valid_identifier(schema, '_')
    {
        return Err(CatalogError::InvalidDatabaseSchema {
            module: module.id.clone(),
            schema: schema.clone(),
        });
    }
    Ok(())
}

fn valid_identifier(value: &str, separator: char) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.ends_with(|character: char| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        })
        && value.chars().all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == separator
        })
}

fn valid_absolute_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && !value.contains(['?', '#'])
        && !value
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_manifests_form_one_valid_catalog() {
        let modules = manifests::ALL
            .into_iter()
            .map(|json| serde_json::from_str(json).unwrap())
            .collect();
        let catalog = ModuleCatalog::new(modules).unwrap();
        assert_eq!(catalog.modules().len(), 5);
        assert_eq!(
            catalog.get("dufs").unwrap().execution,
            ModuleExecution::Service
        );
    }

    #[test]
    fn duplicate_routes_and_ids_are_rejected() {
        let module: ModuleDescriptor = serde_json::from_str(manifests::ALL[0]).unwrap();
        assert_eq!(
            ModuleCatalog::new(vec![module.clone(), module]).unwrap_err(),
            CatalogError::DuplicateId("sunshine".into())
        );

        let first: ModuleDescriptor = serde_json::from_str(manifests::ALL[0]).unwrap();
        let mut second: ModuleDescriptor = serde_json::from_str(manifests::ALL[1]).unwrap();
        second.ui = first.ui.clone();
        assert_eq!(
            ModuleCatalog::new(vec![first, second]).unwrap_err(),
            CatalogError::DuplicateRoute("/modules/sunshine".into())
        );
    }

    #[test]
    fn a_service_cannot_hide_an_invalid_probe_path() {
        let mut module: ModuleDescriptor = serde_json::from_str(manifests::ALL[2]).unwrap();
        module.service.as_mut().unwrap().liveness_path = "http://other-host/health".into();
        assert!(matches!(
            ModuleCatalog::new(vec![module]),
            Err(CatalogError::InvalidServicePath { .. })
        ));
    }
}
