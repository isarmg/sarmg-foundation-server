//! Axum composition for statically linked platform modules.

use axum::Router;
use sarmg_platform_core::{CatalogError, ModuleCatalog, ModuleDescriptor};

pub struct AxumModule<S> {
    pub descriptor: ModuleDescriptor,
    pub console_routes: fn() -> Router<S>,
    pub public_routes: Option<fn() -> Router<S>>,
}

pub struct AssembledModules<S> {
    pub catalog: ModuleCatalog,
    pub console_routes: Router<S>,
    pub public_routes: Router<S>,
}

pub fn assemble<S>(modules: Vec<AxumModule<S>>) -> Result<AssembledModules<S>, CatalogError>
where
    S: Clone + Send + Sync + 'static,
{
    let catalog = ModuleCatalog::new(
        modules
            .iter()
            .map(|module| module.descriptor.clone())
            .collect(),
    )?;
    let mut console_routes = Router::new();
    let mut public_routes = Router::new();
    for module in modules {
        console_routes = console_routes.merge((module.console_routes)());
        if let Some(routes) = module.public_routes {
            public_routes = public_routes.merge(routes());
        }
    }
    Ok(AssembledModules {
        catalog,
        console_routes,
        public_routes,
    })
}
