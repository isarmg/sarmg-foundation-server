use std::collections::{BTreeMap, BTreeSet};

use semver::VersionReq;
use thiserror::Error;

use crate::{ManifestError, PlatformVersions, PluginManifest};

#[derive(Debug, Error)]
pub enum CatalogError {
    #[error("invalid manifest {plugin}: {source}")]
    InvalidManifest {
        plugin: String,
        #[source]
        source: ManifestError,
    },
    #[error("duplicate plugin id: {0}")]
    DuplicateId(String),
    #[error("duplicate service discovery name: {0}")]
    DuplicateService(String),
    #[error("required dependency {dependency} is missing for plugin {plugin}")]
    MissingDependency { plugin: String, dependency: String },
    #[error(
        "plugin {plugin} requires {dependency} {requirement}, but installed version is {actual}"
    )]
    IncompatibleDependency {
        plugin: String,
        dependency: String,
        requirement: String,
        actual: String,
    },
    #[error("plugin dependency graph contains a cycle involving: {0}")]
    DependencyCycle(String),
    #[error("plugin {plugin} is incompatible with {component} {actual}; requires {requirement}")]
    IncompatiblePlatform {
        plugin: String,
        component: &'static str,
        requirement: String,
        actual: String,
    },
}

#[derive(Debug, Clone)]
pub struct PluginCatalog {
    manifests: BTreeMap<String, PluginManifest>,
    activation_order: Vec<String>,
}

impl PluginCatalog {
    pub fn new(manifests: Vec<PluginManifest>) -> Result<Self, CatalogError> {
        let mut by_id = BTreeMap::new();
        let mut service_names = BTreeSet::new();
        for manifest in manifests {
            manifest
                .validate()
                .map_err(|source| CatalogError::InvalidManifest {
                    plugin: manifest.id.clone(),
                    source,
                })?;
            for service in &manifest.services {
                if !service_names.insert(service.name.clone()) {
                    return Err(CatalogError::DuplicateService(service.name.clone()));
                }
            }
            let id = manifest.id.clone();
            if by_id.insert(id.clone(), manifest).is_some() {
                return Err(CatalogError::DuplicateId(id));
            }
        }
        let activation_order = resolve_dependencies(&by_id)?;
        Ok(Self {
            manifests: by_id,
            activation_order,
        })
    }

    pub fn parse_json<'a>(
        manifests: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, CatalogError> {
        let parsed = manifests
            .into_iter()
            .map(|raw| {
                PluginManifest::parse_json(raw).map_err(|source| CatalogError::InvalidManifest {
                    plugin: "<unparsed>".into(),
                    source,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Self::new(parsed)
    }

    pub fn get(&self, id: &str) -> Option<&PluginManifest> {
        self.manifests.get(id)
    }

    pub fn manifests(&self) -> impl Iterator<Item = &PluginManifest> {
        self.manifests.values()
    }

    /// Dependencies always precede dependents; ties are deterministic by plugin id.
    pub fn activation_order(&self) -> impl Iterator<Item = &PluginManifest> {
        self.activation_order.iter().map(|id| &self.manifests[id])
    }

    /// Dependents stop before their dependencies.
    pub fn deactivation_order(&self) -> impl Iterator<Item = &PluginManifest> {
        self.activation_order
            .iter()
            .rev()
            .map(|id| &self.manifests[id])
    }

    pub fn ensure_platform_compatible(
        &self,
        platform: &PlatformVersions,
    ) -> Result<(), CatalogError> {
        for plugin in self.manifests.values() {
            for (component, raw_requirement, actual) in [
                ("core", plugin.compatibility.core.as_str(), &platform.core),
                (
                    "platform_api",
                    plugin.compatibility.platform_api.as_str(),
                    &platform.platform_api,
                ),
                (
                    "plugin_api",
                    plugin.compatibility.plugin_api.as_str(),
                    &platform.plugin_api,
                ),
            ] {
                let requirement = VersionReq::parse(raw_requirement)
                    .expect("requirements were validated when the catalog was built");
                if !requirement.matches(actual) {
                    return Err(CatalogError::IncompatiblePlatform {
                        plugin: plugin.id.clone(),
                        component,
                        requirement: raw_requirement.to_owned(),
                        actual: actual.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}

fn resolve_dependencies(
    manifests: &BTreeMap<String, PluginManifest>,
) -> Result<Vec<String>, CatalogError> {
    let mut dependents: BTreeMap<&str, BTreeSet<&str>> = manifests
        .keys()
        .map(|id| (id.as_str(), BTreeSet::new()))
        .collect();
    let mut indegree: BTreeMap<&str, usize> = manifests.keys().map(|id| (id.as_str(), 0)).collect();

    for manifest in manifests.values() {
        for dependency in &manifest.dependencies {
            let Some(installed) = manifests.get(&dependency.id) else {
                if dependency.optional {
                    continue;
                }
                return Err(CatalogError::MissingDependency {
                    plugin: manifest.id.clone(),
                    dependency: dependency.id.clone(),
                });
            };
            let requirement = VersionReq::parse(&dependency.version)
                .expect("dependency requirements were already validated");
            let actual = installed
                .semantic_version()
                .expect("plugin versions were already validated");
            if !requirement.matches(&actual) {
                return Err(CatalogError::IncompatibleDependency {
                    plugin: manifest.id.clone(),
                    dependency: dependency.id.clone(),
                    requirement: dependency.version.clone(),
                    actual: installed.version.clone(),
                });
            }
            dependents
                .get_mut(dependency.id.as_str())
                .expect("installed dependency has a graph node")
                .insert(manifest.id.as_str());
            *indegree
                .get_mut(manifest.id.as_str())
                .expect("plugin has a graph node") += 1;
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(id, degree)| (*degree == 0).then_some(*id))
        .collect::<BTreeSet<_>>();
    let mut order = Vec::with_capacity(manifests.len());
    while let Some(id) = ready.pop_first() {
        order.push(id.to_owned());
        for dependent in &dependents[id] {
            let degree = indegree
                .get_mut(dependent)
                .expect("dependent has a graph node");
            *degree -= 1;
            if *degree == 0 {
                ready.insert(dependent);
            }
        }
    }
    if order.len() != manifests.len() {
        let cycle = indegree
            .into_iter()
            .filter_map(|(id, degree)| (degree > 0).then_some(id))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(CatalogError::DependencyCycle(cycle));
    }
    Ok(order)
}
