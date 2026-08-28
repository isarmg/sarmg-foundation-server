//! Framework-neutral plugin manifests, validation and dependency resolution.
//!
//! The platform API versions are independent from this crate's release version.

mod catalog;
mod manifest;

pub use catalog::{CatalogError, PluginCatalog};
pub use manifest::*;

pub const MANIFEST_VERSION: u32 = 1;
pub const PLATFORM_API_VERSION: &str = "1.0.0";
pub const PLUGIN_API_VERSION: &str = "1.0.0";
pub const UNION_PLUGIN_ID_ENV: &str = "UNION_PLUGIN_ID";
pub const UNION_PLUGIN_VERSION_ENV: &str = "UNION_PLUGIN_VERSION";
pub const UNION_PLUGIN_BIND_ENV: &str = "UNION_PLUGIN_BIND";
pub const UNION_PLUGIN_PORT_ENV: &str = "UNION_PLUGIN_PORT";
pub const UNION_PLUGIN_CONFIG_ENV: &str = "UNION_PLUGIN_CONFIG";
pub const UNION_PLUGIN_PACKAGE_ROOT_ENV: &str = "UNION_PLUGIN_PACKAGE_ROOT";
pub const UNION_MODULE_PROTOCOL_ENV: &str = "UNION_MODULE_PROTOCOL";
pub const UNION_MODULE_AUDIENCE_ENV: &str = "UNION_MODULE_AUDIENCE";
pub const UNION_MODULE_TOKEN_ENV: &str = "UNION_MODULE_TOKEN";
pub const UNION_MODULE_PREFIX_ENV: &str = "UNION_MODULE_PREFIX";

pub const RESERVED_PROCESS_ENVIRONMENT: [&str; 10] = [
    UNION_PLUGIN_ID_ENV,
    UNION_PLUGIN_VERSION_ENV,
    UNION_PLUGIN_BIND_ENV,
    UNION_PLUGIN_PORT_ENV,
    UNION_PLUGIN_CONFIG_ENV,
    UNION_PLUGIN_PACKAGE_ROOT_ENV,
    UNION_MODULE_PROTOCOL_ENV,
    UNION_MODULE_AUDIENCE_ENV,
    UNION_MODULE_TOKEN_ENV,
    UNION_MODULE_PREFIX_ENV,
];

/// Built-in manifests are migration baselines, not a compile-time allow-list.
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

/// Transitional names for code moving from the old static module catalog.
pub type ModuleCatalog = PluginCatalog;
pub type ModuleDescriptor = PluginManifest;
pub type ModuleHealthState = PluginHealthState;
pub type ModuleInstance = PluginInstance;
