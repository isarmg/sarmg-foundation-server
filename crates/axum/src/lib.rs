//! Axum composition for compile-time Union gateway adapters.
//!
//! The routers assembled here live in Union and proxy to supervised private processes. They are
//! not module business routers and do not share Union's application state with a worker.

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

/// Assemble only the adapters selected by Cargo features in the final Union binary.
///
/// `ModuleCatalog::new` validates that bindings, prefixes, installed binary names and database
/// identities are unique before any route becomes reachable.
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
    use sarmg_platform_core::manifests;

    fn descriptor(raw: &str) -> ModuleDescriptor {
        serde_json::from_str(raw).unwrap()
    }

    #[test]
    fn assembles_gateway_adapters_without_business_router_categories() {
        let assembled = assemble_gateways::<()>(vec![AxumGatewayAdapter {
            descriptor: descriptor(manifests::DUFS),
            gateway_routes: || Router::new().route("/modules/dufs/{*path}", get(|| async {})),
        }])
        .unwrap();
        assert_eq!(assembled.catalog.modules().len(), 1);
        assert_eq!(assembled.catalog.modules()[0].id, "dufs");
    }
}
