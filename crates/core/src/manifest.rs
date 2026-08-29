use std::collections::BTreeSet;

use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::MANIFEST_VERSION;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginManifest {
    pub manifest_version: u32,
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub version: String,
    pub version_metadata: VersionMetadata,
    pub compatibility: Compatibility,
    pub dependencies: Vec<PluginDependency>,
    pub execution: Execution,
    pub backend: BackendContribution,
    pub frontend: FrontendContribution,
    pub permissions: Vec<PermissionDefinition>,
    pub migrations: Vec<DatabaseMigration>,
    pub configuration: ConfigurationDefinition,
    pub health: HealthDefinition,
    pub lifecycle: LifecycleDefinition,
    pub services: Vec<ServiceDefinition>,
    pub events: EventDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionMetadata {
    pub channel: ReleaseChannel,
    pub distribution: PluginDistribution,
    pub license: String,
    #[serde(default)]
    pub source_revision: Option<String>,
    #[serde(default)]
    pub release_notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseChannel {
    Development,
    Beta,
    Stable,
}

/// Runtime discovery is restricted to packages already included in the current release.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginDistribution {
    Bundled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Compatibility {
    pub core: String,
    pub platform_api: String,
    pub plugin_api: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginDependency {
    pub id: String,
    pub version: String,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
pub enum Execution {
    InProcess {
        runtime: InProcessRuntime,
        artifact: String,
        entrypoint: String,
    },
    Process {
        executable: String,
        args: Vec<String>,
        bind: ProcessBind,
    },
    Container {
        image: String,
        digest: String,
    },
    Service {
        service: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InProcessRuntime {
    WasiComponentV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessBind {
    pub host: String,
    /// `0` asks the runtime to allocate an ephemeral port.
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendContribution {
    pub api_version: String,
    pub base_path: String,
    pub service: String,
    pub routes: Vec<BackendRoute>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BackendRoute {
    pub id: String,
    pub path: String,
    pub upstream_path: String,
    pub methods: Vec<HttpMethod>,
    pub auth: RouteAuth,
    pub permission: Option<String>,
    /// Core-enforced ingress bounds. Worker limits may be stricter, but can never extend these
    /// values. The timeout is absolute from request admission until body EOF, so slow drip uploads
    /// cannot retain a Core/Gateway/worker connection indefinitely.
    #[serde(default)]
    pub request_body: RequestBodyPolicy,
}

pub const DEFAULT_ROUTE_REQUEST_MAX_BYTES: u64 = 1024 * 1024;
pub const DEFAULT_ROUTE_REQUEST_TOTAL_TIMEOUT_SECONDS: u32 = 30;
pub const MAX_ROUTE_REQUEST_MAX_BYTES: u64 = 1024 * 1024 * 1024 * 1024;
pub const MAX_ROUTE_REQUEST_TOTAL_TIMEOUT_SECONDS: u32 = 24 * 60 * 60;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestBodyPolicy {
    pub max_bytes: u64,
    pub total_timeout_seconds: u32,
}

impl Default for RequestBodyPolicy {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_ROUTE_REQUEST_MAX_BYTES,
            total_timeout_seconds: DEFAULT_ROUTE_REQUEST_TOTAL_TIMEOUT_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RouteAuth {
    Platform,
    Module,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Delete,
    Get,
    Head,
    Options,
    Patch,
    Post,
    Put,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendContribution {
    pub entry: String,
    pub styles: Vec<String>,
    pub components: Vec<String>,
    pub api_base: String,
    pub routes: Vec<FrontendRoute>,
    pub menu: Vec<MenuEntry>,
}

impl FrontendContribution {
    /// Resolve a bundle-relative asset without allowing a manifest to select another origin.
    pub fn public_asset_path(&self, module_id: &str, asset: &str) -> Option<String> {
        valid_bundle_path(asset).then(|| format!("/modules/{module_id}/assets/{asset}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrontendRoute {
    pub path: String,
    pub component: String,
    pub permission: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuEntry {
    pub id: String,
    pub label: String,
    pub route: String,
    pub permission: String,
    pub order: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PermissionDefinition {
    pub id: String,
    pub description: String,
    pub default_roles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DatabaseMigration {
    pub id: String,
    pub engine: MigrationEngine,
    #[serde(default)]
    pub directory: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationEngine {
    Postgresql,
    Sqlite,
    /// Migrations compiled into the backend (for example Dufs' Rust SQLite migrations).
    Embedded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigurationDefinition {
    /// Bundle-relative JSON Schema path. Configuration is never embedded in the manifest.
    pub schema: String,
    pub version: u32,
    pub secret_fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HealthDefinition {
    Callback {
        liveness_hook: String,
        readiness_hook: String,
        interval_seconds: u32,
        timeout_seconds: u32,
    },
    Http {
        service: String,
        liveness_path: String,
        readiness_path: String,
        interval_seconds: u32,
        timeout_seconds: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifecycleDefinition {
    pub startup_timeout_seconds: u32,
    pub shutdown_timeout_seconds: u32,
    pub restart_policy: RestartPolicy,
    pub max_restarts: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RestartPolicy {
    Never,
    OnFailure,
    Always,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceDefinition {
    pub name: String,
    pub protocol: ServiceProtocol,
    pub visibility: ServiceVisibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceProtocol {
    Http,
    Grpc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceVisibility {
    Module,
    Platform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EventDefinition {
    pub publishes: Vec<PublishedEvent>,
    pub subscribes: Vec<SubscribedEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishedEvent {
    pub topic: String,
    pub version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SubscribedEvent {
    pub topic: String,
    pub version: String,
    pub handler: String,
    pub delivery: DeliveryGuarantee,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryGuarantee {
    AtMostOnce,
    AtLeastOnce,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginHealthState {
    Discovered,
    Installing,
    Starting,
    Available,
    Degraded,
    Backoff,
    Incompatible,
    Stopped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PluginInstance {
    pub id: String,
    pub version: String,
    pub health: PluginHealthState,
    pub message: String,
    pub restart_count: u64,
    pub checked_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlatformVersions {
    pub core: Version,
    pub platform_api: Version,
    pub plugin_api: Version,
}

impl PlatformVersions {
    pub fn parse(core: &str, platform_api: &str, plugin_api: &str) -> Result<Self, semver::Error> {
        Ok(Self {
            core: Version::parse(core)?,
            platform_api: Version::parse(platform_api)?,
            plugin_api: Version::parse(plugin_api)?,
        })
    }
}

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("manifest is not valid JSON")]
    Json(#[from] serde_json::Error),
    #[error("unsupported manifest version {actual}; expected {MANIFEST_VERSION}")]
    UnsupportedManifestVersion { actual: u32 },
    #[error("invalid plugin id: {0}")]
    InvalidId(String),
    #[error("{field} is invalid for plugin {plugin}: {value}")]
    InvalidField {
        plugin: String,
        field: &'static str,
        value: String,
    },
    #[error("{field} contains a duplicate value for plugin {plugin}: {value}")]
    DuplicateValue {
        plugin: String,
        field: &'static str,
        value: String,
    },
    #[error("semantic version is invalid for {plugin}.{field}: {value}")]
    InvalidSemanticVersion {
        plugin: String,
        field: &'static str,
        value: String,
    },
    #[error("unsafe bundle path for {plugin}.{field}: {value}")]
    UnsafeBundlePath {
        plugin: String,
        field: &'static str,
        value: String,
    },
    #[error("permission {permission} referenced by {field} is not declared by plugin {plugin}")]
    UnknownPermission {
        plugin: String,
        field: &'static str,
        permission: String,
    },
    #[error("service {service} referenced by {field} is not declared by plugin {plugin}")]
    UnknownService {
        plugin: String,
        field: &'static str,
        service: String,
    },
    #[error("component {component} referenced by a route is not declared by plugin {plugin}")]
    UnknownComponent { plugin: String, component: String },
    #[error("menu route {route} is not declared by plugin {plugin}")]
    UnknownMenuRoute { plugin: String, route: String },
    #[error("migration {migration} has an invalid {engine:?} storage shape in plugin {plugin}")]
    InvalidMigrationShape {
        plugin: String,
        migration: String,
        engine: MigrationEngine,
    },
    #[error("health kind is incompatible with execution mode for plugin {0}")]
    InvalidHealthKind(String),
}

impl PluginManifest {
    /// Parse with unknown-field rejection at every object, then apply semantic validation.
    pub fn parse_json(input: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(input)?;
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn semantic_version(&self) -> Result<Version, ManifestError> {
        parse_version(&self.id, "version", &self.version)
    }

    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.manifest_version != MANIFEST_VERSION {
            return Err(ManifestError::UnsupportedManifestVersion {
                actual: self.manifest_version,
            });
        }
        if !valid_module_id(&self.id) {
            return Err(ManifestError::InvalidId(self.id.clone()));
        }
        validate_text(&self.id, "display_name", &self.display_name, 128)?;
        validate_text(&self.id, "description", &self.description, 512)?;
        self.semantic_version()?;
        validate_metadata(self)?;
        parse_requirement(&self.id, "compatibility.core", &self.compatibility.core)?;
        parse_requirement(
            &self.id,
            "compatibility.platform_api",
            &self.compatibility.platform_api,
        )?;
        parse_requirement(
            &self.id,
            "compatibility.plugin_api",
            &self.compatibility.plugin_api,
        )?;
        validate_dependencies(self)?;
        validate_execution(self)?;
        validate_permissions(self)?;
        validate_services(self)?;
        validate_backend(self)?;
        validate_frontend(self)?;
        validate_migrations(self)?;
        validate_configuration(self)?;
        validate_health(self)?;
        validate_lifecycle(self)?;
        validate_events(self)
    }
}

fn validate_metadata(manifest: &PluginManifest) -> Result<(), ManifestError> {
    validate_text(
        &manifest.id,
        "version_metadata.license",
        &manifest.version_metadata.license,
        64,
    )?;
    if let Some(revision) = &manifest.version_metadata.source_revision
        && (!(7..=64).contains(&revision.len())
            || !revision.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        return invalid(manifest, "version_metadata.source_revision", revision);
    }
    if let Some(path) = &manifest.version_metadata.release_notes {
        ensure_bundle_path(manifest, "version_metadata.release_notes", path)?;
    }
    Ok(())
}

fn validate_dependencies(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let mut ids = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if !valid_module_id(&dependency.id) || dependency.id == manifest.id {
            return invalid(manifest, "dependencies.id", &dependency.id);
        }
        if !ids.insert(&dependency.id) {
            return duplicate(manifest, "dependencies.id", &dependency.id);
        }
        parse_requirement(&manifest.id, "dependencies.version", &dependency.version)?;
    }
    Ok(())
}

fn validate_execution(manifest: &PluginManifest) -> Result<(), ManifestError> {
    match &manifest.execution {
        Execution::InProcess {
            artifact,
            entrypoint,
            ..
        } => {
            ensure_bundle_path(manifest, "execution.artifact", artifact)?;
            if !artifact.starts_with("backend/")
                || !artifact.ends_with(".wasm")
                || !valid_local_id(entrypoint)
            {
                return invalid(manifest, "execution.entrypoint", entrypoint);
            }
        }
        Execution::Process {
            executable,
            args,
            bind,
        } => {
            ensure_bundle_path(manifest, "execution.executable", executable)?;
            if !executable.starts_with("backend/") {
                return invalid(manifest, "execution.executable", executable);
            }
            for argument in args {
                if argument.is_empty()
                    || argument.len() > 4096
                    || argument.chars().any(char::is_control)
                {
                    return invalid(manifest, "execution.args", argument);
                }
            }
            if !matches!(bind.host.as_str(), "127.0.0.1" | "::1") {
                return invalid(manifest, "execution.bind.host", &bind.host);
            }
        }
        Execution::Container { image, digest } => {
            if !valid_container_image(image) {
                return invalid(manifest, "execution.image", image);
            }
            if !valid_sha256_digest(digest) {
                return invalid(manifest, "execution.digest", digest);
            }
        }
        Execution::Service { service } => {
            if !valid_dotted_identifier(service) {
                return invalid(manifest, "execution.service", service);
            }
        }
    }
    Ok(())
}

fn validate_permissions(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let mut ids = BTreeSet::new();
    let prefix = format!("{}.", manifest.id);
    for permission in &manifest.permissions {
        if !permission.id.starts_with(&prefix) || !valid_dotted_identifier(&permission.id) {
            return invalid(manifest, "permissions.id", &permission.id);
        }
        if !ids.insert(&permission.id) {
            return duplicate(manifest, "permissions.id", &permission.id);
        }
        validate_text(
            &manifest.id,
            "permissions.description",
            &permission.description,
            256,
        )?;
        let mut roles = BTreeSet::new();
        for role in &permission.default_roles {
            if !valid_local_id(role) {
                return invalid(manifest, "permissions.default_roles", role);
            }
            if !roles.insert(role) {
                return duplicate(manifest, "permissions.default_roles", role);
            }
        }
    }
    Ok(())
}

fn validate_services(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let mut names = BTreeSet::new();
    let prefix = format!("{}.", manifest.id);
    for service in &manifest.services {
        if !service.name.starts_with(&prefix) || !valid_dotted_identifier(&service.name) {
            return invalid(manifest, "services.name", &service.name);
        }
        if !names.insert(&service.name) {
            return duplicate(manifest, "services.name", &service.name);
        }
    }
    if let Execution::Service { service } = &manifest.execution {
        ensure_service(manifest, "execution.service", service)?;
    }
    Ok(())
}

fn validate_backend(manifest: &PluginManifest) -> Result<(), ManifestError> {
    if !valid_api_version(&manifest.backend.api_version) {
        return invalid(
            manifest,
            "backend.api_version",
            &manifest.backend.api_version,
        );
    }
    let expected_base = format!("/api/modules/{}", manifest.id);
    if manifest.backend.base_path != expected_base {
        return invalid(manifest, "backend.base_path", &manifest.backend.base_path);
    }
    ensure_service(manifest, "backend.service", &manifest.backend.service)?;
    let permissions = permission_ids(manifest);
    let mut ids = BTreeSet::new();
    let mut signatures = BTreeSet::new();
    let mut validated_routes: Vec<&BackendRoute> = Vec::new();
    for route in &manifest.backend.routes {
        if !valid_local_id(&route.id) {
            return invalid(manifest, "backend.routes.id", &route.id);
        }
        if !ids.insert(&route.id) {
            return duplicate(manifest, "backend.routes.id", &route.id);
        }
        // A module-local root is safe here: the gateway mounts it below the
        // manifest's canonical `/api/modules/{id}` base path.  File-browser
        // modules also need it so their relative assets and redirects remain
        // under the same module namespace.
        if !valid_route_path(&route.path) {
            return invalid(manifest, "backend.routes.path", &route.path);
        }
        if !valid_route_path(&route.upstream_path)
            || !same_route_parameters(&route.path, &route.upstream_path)
        {
            return invalid(
                manifest,
                "backend.routes.upstream_path",
                &route.upstream_path,
            );
        }
        if route.methods.is_empty() {
            return invalid(manifest, "backend.routes.methods", "<empty>");
        }
        let mut methods = BTreeSet::new();
        for method in &route.methods {
            if !methods.insert(*method) {
                return duplicate(manifest, "backend.routes.methods", &format!("{method:?}"));
            }
            if !signatures.insert((route.path.as_str(), *method)) {
                return duplicate(
                    manifest,
                    "backend.routes.path+method",
                    &format!("{} {method:?}", route.path),
                );
            }
        }
        match (route.auth, route.permission.as_deref()) {
            (RouteAuth::Platform, Some(permission)) => ensure_permission(
                manifest,
                &permissions,
                "backend.routes.permission",
                permission,
            )?,
            (RouteAuth::Module, None) => {}
            _ => return invalid(manifest, "backend.routes.auth", &route.id),
        }
        if route.request_body.max_bytes == 0
            || route.request_body.max_bytes > MAX_ROUTE_REQUEST_MAX_BYTES
        {
            return invalid(
                manifest,
                "backend.routes.request_body.max_bytes",
                &route.request_body.max_bytes.to_string(),
            );
        }
        if route.request_body.total_timeout_seconds == 0
            || route.request_body.total_timeout_seconds > MAX_ROUTE_REQUEST_TOTAL_TIMEOUT_SECONDS
        {
            return invalid(
                manifest,
                "backend.routes.request_body.total_timeout_seconds",
                &route.request_body.total_timeout_seconds.to_string(),
            );
        }
        for previous in &validated_routes {
            let shared_method = route
                .methods
                .iter()
                .any(|method| previous.methods.contains(method));
            if shared_method
                && route_patterns_overlap(&route.path, &previous.path)
                && route_specificity(&route.path) == route_specificity(&previous.path)
            {
                return invalid(
                    manifest,
                    "backend.routes.path+method",
                    &format!(
                        "{} overlaps {} with equal specificity",
                        route.id, previous.id
                    ),
                );
            }
        }
        validated_routes.push(route);
    }
    Ok(())
}

fn validate_frontend(manifest: &PluginManifest) -> Result<(), ManifestError> {
    ensure_bundle_path(manifest, "frontend.entry", &manifest.frontend.entry)?;
    if !manifest.frontend.entry.ends_with(".js") {
        return invalid(manifest, "frontend.entry", &manifest.frontend.entry);
    }
    let mut assets = BTreeSet::from([manifest.frontend.entry.as_str()]);
    for style in &manifest.frontend.styles {
        ensure_bundle_path(manifest, "frontend.styles", style)?;
        if !style.ends_with(".css") {
            return invalid(manifest, "frontend.styles", style);
        }
        if !assets.insert(style) {
            return duplicate(manifest, "frontend.assets", style);
        }
    }
    let expected_api = format!("/api/modules/{}", manifest.id);
    if manifest.frontend.api_base != expected_api {
        return invalid(manifest, "frontend.api_base", &manifest.frontend.api_base);
    }
    let mut components: BTreeSet<&str> = BTreeSet::new();
    for component in &manifest.frontend.components {
        if !valid_component_name(component) {
            return invalid(manifest, "frontend.components", component);
        }
        if !components.insert(component.as_str()) {
            return duplicate(manifest, "frontend.components", component);
        }
    }
    if components.is_empty() {
        return invalid(manifest, "frontend.components", "<empty>");
    }
    let permissions = permission_ids(manifest);
    let route_prefix = format!("/modules/{}", manifest.id);
    let mut routes: BTreeSet<&str> = BTreeSet::new();
    for route in &manifest.frontend.routes {
        if !valid_route_path(&route.path)
            || !(route.path == route_prefix || route.path.starts_with(&format!("{route_prefix}/")))
        {
            return invalid(manifest, "frontend.routes.path", &route.path);
        }
        if !routes.insert(route.path.as_str()) {
            return duplicate(manifest, "frontend.routes.path", &route.path);
        }
        if !components.contains(route.component.as_str()) {
            return Err(ManifestError::UnknownComponent {
                plugin: manifest.id.clone(),
                component: route.component.clone(),
            });
        }
        ensure_permission(
            manifest,
            &permissions,
            "frontend.routes.permission",
            &route.permission,
        )?;
    }
    let mut menu_ids = BTreeSet::new();
    let mut menu_orders = BTreeSet::new();
    for menu in &manifest.frontend.menu {
        if !valid_local_id(&menu.id) {
            return invalid(manifest, "frontend.menu.id", &menu.id);
        }
        if !menu_ids.insert(&menu.id) {
            return duplicate(manifest, "frontend.menu.id", &menu.id);
        }
        if !menu_orders.insert(menu.order) {
            return duplicate(manifest, "frontend.menu.order", &menu.order.to_string());
        }
        validate_text(&manifest.id, "frontend.menu.label", &menu.label, 64)?;
        if !routes.contains(menu.route.as_str()) {
            return Err(ManifestError::UnknownMenuRoute {
                plugin: manifest.id.clone(),
                route: menu.route.clone(),
            });
        }
        ensure_permission(
            manifest,
            &permissions,
            "frontend.menu.permission",
            &menu.permission,
        )?;
    }
    Ok(())
}

fn validate_migrations(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let mut ids = BTreeSet::new();
    let mut schemas = BTreeSet::new();
    for migration in &manifest.migrations {
        if !valid_local_id(&migration.id) {
            return invalid(manifest, "migrations.id", &migration.id);
        }
        if !ids.insert(&migration.id) {
            return duplicate(manifest, "migrations.id", &migration.id);
        }
        let valid_shape = match migration.engine {
            MigrationEngine::Postgresql => {
                migration
                    .directory
                    .as_deref()
                    .is_some_and(valid_bundle_path)
                    && migration
                        .schema
                        .as_deref()
                        .is_some_and(valid_postgres_identifier)
            }
            MigrationEngine::Sqlite => {
                migration
                    .directory
                    .as_deref()
                    .is_some_and(valid_bundle_path)
                    && migration.schema.is_none()
            }
            MigrationEngine::Embedded => {
                migration.directory.is_none() && migration.schema.is_none()
            }
        };
        if !valid_shape {
            return Err(ManifestError::InvalidMigrationShape {
                plugin: manifest.id.clone(),
                migration: migration.id.clone(),
                engine: migration.engine,
            });
        }
        if let Some(schema) = &migration.schema
            && !schemas.insert(schema)
        {
            return duplicate(manifest, "migrations.schema", schema);
        }
    }
    Ok(())
}

fn validate_configuration(manifest: &PluginManifest) -> Result<(), ManifestError> {
    ensure_bundle_path(
        manifest,
        "configuration.schema",
        &manifest.configuration.schema,
    )?;
    if !manifest.configuration.schema.starts_with("config/")
        || !manifest.configuration.schema.ends_with(".json")
        || manifest.configuration.version == 0
    {
        return invalid(manifest, "configuration", &manifest.configuration.schema);
    }
    let mut fields = BTreeSet::new();
    for field in &manifest.configuration.secret_fields {
        if !valid_json_pointer(field) {
            return invalid(manifest, "configuration.secret_fields", field);
        }
        if !fields.insert(field) {
            return duplicate(manifest, "configuration.secret_fields", field);
        }
    }
    Ok(())
}

fn validate_health(manifest: &PluginManifest) -> Result<(), ManifestError> {
    match (&manifest.execution, &manifest.health) {
        (
            Execution::InProcess { .. },
            HealthDefinition::Callback {
                liveness_hook,
                readiness_hook,
                interval_seconds,
                timeout_seconds,
            },
        ) => {
            if !valid_local_id(liveness_hook)
                || !valid_local_id(readiness_hook)
                || liveness_hook == readiness_hook
            {
                return invalid(manifest, "health.callback", liveness_hook);
            }
            validate_probe_timing(manifest, *interval_seconds, *timeout_seconds)
        }
        (
            Execution::Process { .. } | Execution::Container { .. } | Execution::Service { .. },
            HealthDefinition::Http {
                service,
                liveness_path,
                readiness_path,
                interval_seconds,
                timeout_seconds,
            },
        ) => {
            ensure_service(manifest, "health.service", service)?;
            if !valid_route_path(liveness_path)
                || !valid_route_path(readiness_path)
                || liveness_path == readiness_path
            {
                return invalid(manifest, "health.http", liveness_path);
            }
            validate_probe_timing(manifest, *interval_seconds, *timeout_seconds)
        }
        _ => Err(ManifestError::InvalidHealthKind(manifest.id.clone())),
    }
}

fn validate_probe_timing(
    manifest: &PluginManifest,
    interval: u32,
    timeout: u32,
) -> Result<(), ManifestError> {
    if interval == 0 || interval > 3600 || timeout == 0 || timeout > interval {
        return invalid(manifest, "health.timing", &format!("{interval}/{timeout}"));
    }
    Ok(())
}

fn validate_lifecycle(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let lifecycle = &manifest.lifecycle;
    if lifecycle.startup_timeout_seconds == 0
        || lifecycle.startup_timeout_seconds > 3600
        || lifecycle.shutdown_timeout_seconds == 0
        || lifecycle.shutdown_timeout_seconds > 3600
        || (lifecycle.restart_policy == RestartPolicy::Never && lifecycle.max_restarts != 0)
        || (lifecycle.restart_policy != RestartPolicy::Never && lifecycle.max_restarts == 0)
    {
        return invalid(manifest, "lifecycle", "invalid timeout/restart combination");
    }
    Ok(())
}

fn validate_events(manifest: &PluginManifest) -> Result<(), ManifestError> {
    let mut published = BTreeSet::new();
    let publish_prefix = format!("{}.", manifest.id);
    for event in &manifest.events.publishes {
        if !event.topic.starts_with(&publish_prefix)
            || !valid_dotted_identifier(&event.topic)
            || event.version == 0
        {
            return invalid(manifest, "events.publishes", &event.topic);
        }
        if !published.insert((&event.topic, event.version)) {
            return duplicate(manifest, "events.publishes", &event.topic);
        }
    }
    let mut subscribed = BTreeSet::new();
    for event in &manifest.events.subscribes {
        if !valid_dotted_identifier(&event.topic) || !valid_local_id(&event.handler) {
            return invalid(manifest, "events.subscribes", &event.topic);
        }
        parse_requirement(&manifest.id, "events.subscribes.version", &event.version)?;
        if !subscribed.insert((&event.topic, &event.handler)) {
            return duplicate(manifest, "events.subscribes", &event.topic);
        }
    }
    Ok(())
}

fn ensure_permission(
    manifest: &PluginManifest,
    permissions: &BTreeSet<&str>,
    field: &'static str,
    permission: &str,
) -> Result<(), ManifestError> {
    if !permissions.contains(permission) {
        return Err(ManifestError::UnknownPermission {
            plugin: manifest.id.clone(),
            field,
            permission: permission.to_owned(),
        });
    }
    Ok(())
}

fn ensure_service(
    manifest: &PluginManifest,
    field: &'static str,
    service: &str,
) -> Result<(), ManifestError> {
    if !manifest.services.iter().any(|item| item.name == service) {
        return Err(ManifestError::UnknownService {
            plugin: manifest.id.clone(),
            field,
            service: service.to_owned(),
        });
    }
    Ok(())
}

fn permission_ids(manifest: &PluginManifest) -> BTreeSet<&str> {
    manifest
        .permissions
        .iter()
        .map(|item| item.id.as_str())
        .collect()
}

pub(crate) fn parse_requirement(
    plugin: &str,
    field: &'static str,
    value: &str,
) -> Result<VersionReq, ManifestError> {
    VersionReq::parse(value).map_err(|_| ManifestError::InvalidSemanticVersion {
        plugin: plugin.to_owned(),
        field,
        value: value.to_owned(),
    })
}

fn parse_version(plugin: &str, field: &'static str, value: &str) -> Result<Version, ManifestError> {
    Version::parse(value).map_err(|_| ManifestError::InvalidSemanticVersion {
        plugin: plugin.to_owned(),
        field,
        value: value.to_owned(),
    })
}

fn validate_text(
    plugin: &str,
    field: &'static str,
    value: &str,
    max: usize,
) -> Result<(), ManifestError> {
    if value.trim().is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(ManifestError::InvalidField {
            plugin: plugin.to_owned(),
            field,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn ensure_bundle_path(
    manifest: &PluginManifest,
    field: &'static str,
    path: &str,
) -> Result<(), ManifestError> {
    if !valid_bundle_path(path) {
        return Err(ManifestError::UnsafeBundlePath {
            plugin: manifest.id.clone(),
            field,
            value: path.to_owned(),
        });
    }
    Ok(())
}

fn invalid<T>(
    manifest: &PluginManifest,
    field: &'static str,
    value: &str,
) -> Result<T, ManifestError> {
    Err(ManifestError::InvalidField {
        plugin: manifest.id.clone(),
        field,
        value: value.to_owned(),
    })
}

fn duplicate<T>(
    manifest: &PluginManifest,
    field: &'static str,
    value: &str,
) -> Result<T, ManifestError> {
    Err(ManifestError::DuplicateValue {
        plugin: manifest.id.clone(),
        field,
        value: value.to_owned(),
    })
}

fn valid_module_id(value: &str) -> bool {
    valid_hyphen_identifier(value, 64)
}

fn valid_local_id(value: &str) -> bool {
    valid_hyphen_identifier(value, 64)
}

fn valid_hyphen_identifier(value: &str, max: usize) -> bool {
    !value.is_empty()
        && value.len() <= max
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .last()
            .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.contains("--")
}

fn valid_dotted_identifier(value: &str) -> bool {
    !value.is_empty() && value.len() <= 128 && value.split('.').all(valid_local_id)
}

fn valid_api_version(value: &str) -> bool {
    value.len() <= 16
        && value.strip_prefix('v').is_some_and(|number| {
            !number.is_empty()
                && number.bytes().all(|byte| byte.is_ascii_digit())
                && !number.starts_with('0')
        })
}

fn valid_route_path(value: &str) -> bool {
    value.starts_with('/')
        && !value.starts_with("//")
        && value.len() <= 512
        && !value.contains(['?', '#', '\\', '%'])
        && !value.chars().any(char::is_control)
        && !value
            .split('/')
            .any(|segment| matches!(segment, "." | ".."))
}

/// Ordering used by the gateway's most-specific route selection. Static segments win first,
/// followed by the number of non-wildcard segments and finally an exact (non-wildcard) ending.
pub fn route_specificity(value: &str) -> (usize, usize, bool) {
    let segments = value
        .split('/')
        .skip(1)
        .filter(|segment| !segment.is_empty());
    let mut static_segments = 0;
    let mut non_wildcard_segments = 0;
    let mut wildcard = false;
    for segment in segments {
        if segment.starts_with("{*") {
            wildcard = true;
        } else {
            non_wildcard_segments += 1;
            if !(segment.starts_with('{') && segment.ends_with('}')) {
                static_segments += 1;
            }
        }
    }
    (static_segments, non_wildcard_segments, !wildcard)
}

fn route_patterns_overlap(left: &str, right: &str) -> bool {
    let left = left
        .split('/')
        .skip(1)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let right = right
        .split('/')
        .skip(1)
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let mut index = 0;
    loop {
        match (left.get(index), right.get(index)) {
            (None, None) => return true,
            (Some(segment), None) | (None, Some(segment)) => {
                return segment.starts_with("{*");
            }
            (Some(left), Some(right)) => {
                if left.starts_with("{*") || right.starts_with("{*") {
                    return true;
                }
                let left_parameter = left.starts_with('{');
                let right_parameter = right.starts_with('{');
                if !left_parameter && !right_parameter && left != right {
                    return false;
                }
            }
        }
        index += 1;
    }
}

fn same_route_parameters(left: &str, right: &str) -> bool {
    matches!(
        (route_parameters(left), route_parameters(right)),
        (Some(left), Some(right)) if left == right
    )
}

fn route_parameters(value: &str) -> Option<BTreeSet<(&str, bool)>> {
    let segments = value.split('/').skip(1).collect::<Vec<_>>();
    let mut parameters = BTreeSet::new();
    for (index, segment) in segments.iter().enumerate() {
        if !segment.contains(['{', '}']) {
            continue;
        }
        let inner = segment.strip_prefix('{')?.strip_suffix('}')?;
        let (wildcard, name) = inner
            .strip_prefix('*')
            .map_or((false, inner), |name| (true, name));
        if !valid_route_parameter(name) || (wildcard && index + 1 != segments.len()) {
            return None;
        }
        if !parameters.insert((name, wildcard)) {
            return None;
        }
    }
    Some(parameters)
}

fn valid_route_parameter(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_')
        })
}

pub(crate) fn valid_bundle_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.starts_with('/')
        && !value.ends_with('/')
        && !value.contains(['\\', ':', '?', '#'])
        && !value.chars().any(char::is_control)
        && value
            .split('/')
            .all(|segment| !segment.is_empty() && !matches!(segment, "." | ".."))
}

fn valid_component_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_uppercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn valid_container_image(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 255
        && !value.contains(['@', ' ', '\\'])
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._/-:".contains(&byte))
}

fn valid_sha256_digest(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn valid_postgres_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 63
        && value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_lowercase())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn valid_json_pointer(value: &str) -> bool {
    value.starts_with('/')
        && value.len() <= 255
        && !value.chars().any(char::is_control)
        && !value.split('/').skip(1).any(|segment| {
            segment.is_empty()
                || segment
                    .as_bytes()
                    .windows(2)
                    .any(|pair| pair[0] == b'~' && !matches!(pair[1], b'0' | b'1'))
                || segment.ends_with('~')
        })
}
