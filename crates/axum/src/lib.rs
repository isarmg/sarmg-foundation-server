//! Optional Axum adapter for routes contributed by release-bundled runtime plugins.
//!
//! The framework-neutral manifest and SDK do not depend on Axum. This crate is only a host-side
//! bridge and must be called again when the active runtime catalog changes.

use axum::Router;
use sarmg_platform_core::{CatalogError, ModuleCatalog, ModuleDescriptor};

pub struct AxumGatewayAdapter<S> {
    pub descriptor: ModuleDescriptor,
    pub gateway_routes: fn() -> Router<S>,
}

pub struct AssembledGateways<S> {
    pub catalog: ModuleCatalog,
    pub gateway_routes: Router<S>,
}

/// Assemble adapters for the validated active runtime catalog.
///
/// `ModuleCatalog::new` validates compatibility, dependencies, paths, permissions and service
/// names before any route becomes reachable.
pub fn assemble_gateways<S>(
    adapters: Vec<AxumGatewayAdapter<S>>,
) -> Result<AssembledGateways<S>, CatalogError>
where
    S: Clone + Send + Sync + 'static,
{
    let catalog = ModuleCatalog::new(
        adapters
            .iter()
            .map(|adapter| adapter.descriptor.clone())
            .collect(),
    )?;
    let mut gateway_routes = Router::new();
    for adapter in adapters {
        gateway_routes = gateway_routes.merge((adapter.gateway_routes)());
    }
    Ok(AssembledGateways {
        catalog,
        gateway_routes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::routing::get;

    const EXAMPLE_MANIFEST: &str = include_str!("../../../modules/dufs.json");

    fn descriptor(raw: &str) -> ModuleDescriptor {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn assembles_gateway_adapters_without_business_router_categories() {
        let assembled = assemble_gateways::<()>(vec![AxumGatewayAdapter {
            descriptor: descriptor(EXAMPLE_MANIFEST),
            gateway_routes: || {
                Router::new().route("/api/modules/dufs/files/{*path}", get(|| async {}))
            },
        }])
        .unwrap();
        assert_eq!(assembled.catalog.manifests().count(), 1);
        assert_eq!(assembled.catalog.get("dufs").unwrap().id, "dufs");
    }
}
