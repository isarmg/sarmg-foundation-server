use std::collections::BTreeSet;

use sarmg_platform_core::{
    CatalogError, EnvironmentBinding, Execution, HealthDefinition, ManifestError, MigrationEngine,
    PLATFORM_API_VERSION, PLUGIN_API_VERSION, PlatformVersions, PluginCatalog, PluginDependency,
    PluginManifest, RouteAuth,
};

const PROCESS_FIXTURE: &str = include_str!("../../../tests/fixtures/process-module.json");

fn fixture(id: &str, version: &str) -> PluginManifest {
    let raw = PROCESS_FIXTURE.replace("fixture-module", id).replacen(
        "\"version\": \"1.2.3\"",
        &format!("\"version\": \"{version}\""),
        1,
    );
    PluginManifest::parse_json(&raw).unwrap()
}

fn fixtures() -> Vec<PluginManifest> {
    let mut manifests = [
        ("fixture-control", "1.0.0"),
        ("fixture-host", "1.1.0"),
        ("fixture-media", "1.2.0"),
        ("fixture-library", "1.3.0"),
        ("fixture-storage", "1.4.0"),
    ]
    .into_iter()
    .map(|(id, version)| fixture(id, version))
    .collect::<Vec<_>>();

    let storage = manifests
        .iter_mut()
        .find(|manifest| manifest.id == "fixture-storage")
        .unwrap();
    let Execution::Process { bind, .. } = &mut storage.execution else {
        unreachable!()
    };
    bind.port = 18103;
    storage.migrations[0].engine = MigrationEngine::Embedded;
    storage.migrations[0].directory = None;
    storage.migrations[0].schema = None;
    storage.validate().unwrap();

    manifests
}

#[test]
fn complete_process_package_example_uses_the_same_contract() {
    PluginManifest::parse_json(include_str!(
        "../../../examples/process-module/manifest.json"
    ))
    .unwrap();
}

#[test]
fn five_module_style_fixture_is_a_valid_process_catalog() {
    let catalog = PluginCatalog::new(fixtures()).unwrap();
    assert_eq!(catalog.manifests().count(), 5);
    assert_eq!(PLATFORM_API_VERSION, "1.0.0");
    assert_eq!(PLUGIN_API_VERSION, "1.0.0");
    assert_eq!(catalog.get("fixture-storage").unwrap().version, "1.4.0");
    for plugin in catalog.manifests() {
        let Execution::Process {
            executable, bind, ..
        } = &plugin.execution
        else {
            panic!("{} is not a process plugin", plugin.id);
        };
        assert!(executable.starts_with("backend/"));
        assert_eq!(bind.host, "127.0.0.1");
        assert!(bind.port == 0 || (plugin.id == "fixture-storage" && bind.port == 18103));
        assert_eq!(
            plugin.backend.base_path,
            format!("/api/modules/{}", plugin.id)
        );
        assert_eq!(plugin.frontend.api_base, plugin.backend.base_path);
        assert!(
            plugin
                .frontend
                .public_asset_path(&plugin.id, &plugin.frontend.entry)
                .unwrap()
                .starts_with(&format!("/modules/{}/assets/", plugin.id))
        );
    }
}

#[test]
fn json_schema_tracks_rust_contract_and_all_execution_modes() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../contracts/plugin-manifest-v1.schema.json"
    ))
    .unwrap();
    let required = schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect::<BTreeSet<_>>();
    for field in [
        "compatibility",
        "execution",
        "backend",
        "frontend",
        "permissions",
        "migrations",
        "configuration",
        "health",
        "lifecycle",
        "services",
        "events",
    ] {
        assert!(required.contains(field));
    }
    let serialized = schema.to_string();
    for mode in ["in_process", "process", "container", "service"] {
        assert!(serialized.contains(mode));
    }
    for field in ["styles", "components", "platform_api", "plugin_api"] {
        assert!(serialized.contains(field));
    }
    assert!(serialized.contains("request_body"));
}

#[test]
fn route_request_body_policy_is_bounded_and_defaults_securely() {
    let mut plugin = fixture("fixture-body-policy", "1.0.0");
    assert_eq!(plugin.backend.routes[0].request_body.max_bytes, 1024 * 1024);
    assert_eq!(
        plugin.backend.routes[0].request_body.total_timeout_seconds,
        30
    );

    plugin.backend.routes[0].request_body.max_bytes = 0;
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField { .. })
    ));
    plugin.backend.routes[0].request_body.max_bytes = 1024;
    plugin.backend.routes[0].request_body.total_timeout_seconds = 86_401;
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField { .. })
    ));
}

#[test]
fn strict_parser_rejects_unknown_fields() {
    let raw = PROCESS_FIXTURE.replacen(
        "\"manifest_version\": 1,",
        "\"manifest_version\": 1, \"surprise\": true,",
        1,
    );
    assert!(matches!(
        PluginManifest::parse_json(&raw),
        Err(ManifestError::Json(_))
    ));

    let missing_required = PROCESS_FIXTURE.replace("  \"dependencies\": [],\n", "");
    assert!(matches!(
        PluginManifest::parse_json(&missing_required),
        Err(ManifestError::Json(_))
    ));
}

#[test]
fn dependency_order_is_deterministic_and_topological() {
    let mut plugins = fixtures();
    let host_version = plugins
        .iter()
        .find(|plugin| plugin.id == "fixture-host")
        .unwrap()
        .version
        .clone();
    plugins
        .iter_mut()
        .find(|plugin| plugin.id == "fixture-control")
        .unwrap()
        .dependencies
        .push(PluginDependency {
            id: "fixture-host".into(),
            version: format!("={host_version}"),
            optional: false,
        });
    let catalog = PluginCatalog::new(plugins).unwrap();
    let order = catalog
        .activation_order()
        .map(|plugin| plugin.id.as_str())
        .collect::<Vec<_>>();
    assert!(
        order.iter().position(|id| *id == "fixture-host").unwrap()
            < order
                .iter()
                .position(|id| *id == "fixture-control")
                .unwrap()
    );
    let shutdown = catalog
        .deactivation_order()
        .map(|plugin| plugin.id.as_str())
        .collect::<Vec<_>>();
    assert!(
        shutdown
            .iter()
            .position(|id| *id == "fixture-control")
            .unwrap()
            < shutdown
                .iter()
                .position(|id| *id == "fixture-host")
                .unwrap()
    );
}

#[test]
fn cycles_missing_dependencies_and_version_mismatches_are_rejected() {
    let mut plugins = fixtures();
    let media_index = plugins
        .iter()
        .position(|plugin| plugin.id == "fixture-media")
        .unwrap();
    let library_index = plugins
        .iter()
        .position(|plugin| plugin.id == "fixture-library")
        .unwrap();
    plugins[media_index].dependencies.push(PluginDependency {
        id: "fixture-library".into(),
        version: "*".into(),
        optional: false,
    });
    plugins[library_index].dependencies.push(PluginDependency {
        id: "fixture-media".into(),
        version: "*".into(),
        optional: false,
    });
    assert!(matches!(
        PluginCatalog::new(plugins),
        Err(CatalogError::DependencyCycle(_))
    ));

    let mut plugin = fixtures().remove(0);
    plugin.dependencies.push(PluginDependency {
        id: "missing-plugin".into(),
        version: "^1.0.0".into(),
        optional: false,
    });
    assert!(matches!(
        PluginCatalog::new(vec![plugin]),
        Err(CatalogError::MissingDependency { .. })
    ));

    let mut plugins = fixtures();
    let dependency_id = plugins[1].id.clone();
    plugins[0].dependencies.push(PluginDependency {
        id: dependency_id,
        version: ">=9.0.0".into(),
        optional: false,
    });
    assert!(matches!(
        PluginCatalog::new(plugins),
        Err(CatalogError::IncompatibleDependency { .. })
    ));
}

#[test]
fn optional_missing_dependency_does_not_block_activation() {
    let mut plugin = fixtures().remove(0);
    plugin.dependencies.push(PluginDependency {
        id: "optional-plugin".into(),
        version: "^1.0.0".into(),
        optional: true,
    });
    PluginCatalog::new(vec![plugin]).unwrap();
}

#[test]
fn compatibility_ranges_are_enforced_before_activation() {
    let catalog = PluginCatalog::new(fixtures()).unwrap();
    catalog
        .ensure_platform_compatible(
            &PlatformVersions::parse("0.5.0", PLATFORM_API_VERSION, PLUGIN_API_VERSION).unwrap(),
        )
        .unwrap();
    assert!(matches!(
        catalog.ensure_platform_compatible(
            &PlatformVersions::parse("2.0.0", PLATFORM_API_VERSION, PLUGIN_API_VERSION).unwrap()
        ),
        Err(CatalogError::IncompatiblePlatform {
            component: "core",
            ..
        })
    ));
}

#[test]
fn traversal_foreign_routes_permissions_and_components_fail_closed() {
    let mut plugin = fixtures().remove(2);
    plugin.frontend.entry = "../remote.js".into();
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::UnsafeBundlePath { .. })
    ));

    let mut plugin = fixtures().remove(2);
    plugin.frontend.routes[0].path = "/modules/other".into();
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField { .. })
    ));

    let mut plugin = fixtures().remove(2);
    plugin.backend.routes[0].auth = sarmg_platform_core::RouteAuth::Platform;
    plugin.backend.routes[0].permission = Some("other.admin".into());
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::UnknownPermission { .. })
    ));

    let mut plugin = fixtures().remove(2);
    plugin.frontend.routes[0].component = "Undeclared".into();
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::UnknownComponent { .. })
    ));
}

#[test]
fn route_rewrites_auth_modes_and_reserved_environment_fail_closed() {
    let mut plugin = fixtures().remove(0);
    plugin.backend.routes[0].upstream_path = "/internal/{other}".into();
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField {
            field: "backend.routes.upstream_path",
            ..
        })
    ));

    let mut plugin = fixtures().remove(0);
    plugin.backend.routes[0].auth = RouteAuth::Platform;
    plugin.backend.routes[0].permission = None;
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField {
            field: "backend.routes.auth",
            ..
        })
    ));

    let mut plugin = fixtures().remove(0);
    let Execution::Process { environment, .. } = &mut plugin.execution else {
        unreachable!()
    };
    environment.push(EnvironmentBinding {
        name: "UNION_PLUGIN_CONFIG".into(),
        config_pointer: "/secret".into(),
    });
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField {
            field: "execution.environment.name",
            ..
        })
    ));
}

#[test]
fn encoded_and_equally_specific_overlapping_routes_fail_closed() {
    let mut plugin = fixtures()
        .into_iter()
        .find(|plugin| plugin.id == "fixture-storage")
        .unwrap();
    let mut first = plugin.backend.routes[1].clone();
    first.id = "ambiguous-first".into();
    first.path = "/lookup/{value}".into();
    first.upstream_path = "/lookup/{value}".into();
    let mut second = first.clone();
    second.id = "ambiguous-second".into();
    second.path = "/lookup/{other}".into();
    second.upstream_path = "/lookup/{other}".into();
    plugin.backend.routes.extend([first, second]);
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField {
            field: "backend.routes.path+method",
            ..
        })
    ));

    let mut plugin = fixtures().remove(0);
    plugin.backend.routes[0].path = "/%2e%2e/{*path}".into();
    plugin.backend.routes[0].upstream_path = "/%2e%2e/{*path}".into();
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidField {
            field: "backend.routes.path",
            ..
        })
    ));

    assert!(
        sarmg_platform_core::route_specificity("/resources/{resource_id}/actions")
            > sarmg_platform_core::route_specificity("/resources/{*path}")
    );
    assert!(
        sarmg_platform_core::route_specificity("/")
            > sarmg_platform_core::route_specificity("/{*path}")
    );
}

#[test]
fn execution_modes_and_health_kinds_are_paired() {
    let mut plugin = fixtures().remove(0);
    plugin.execution = Execution::InProcess {
        runtime: sarmg_platform_core::InProcessRuntime::WasiComponentV1,
        artifact: "backend/fixture-module.wasm".into(),
        entrypoint: "activate".into(),
    };
    assert!(matches!(
        plugin.validate(),
        Err(ManifestError::InvalidHealthKind(_))
    ));
    plugin.health = HealthDefinition::Callback {
        liveness_hook: "liveness".into(),
        readiness_hook: "readiness".into(),
        interval_seconds: 10,
        timeout_seconds: 2,
    };
    plugin.validate().unwrap();

    plugin.execution = Execution::Container {
        image: "registry.example/fixture:1.0.0".into(),
        digest: "sha256:ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".into(),
    };
    plugin.health = HealthDefinition::Http {
        service: "fixture-control.api".into(),
        liveness_path: "/health/live".into(),
        readiness_path: "/health/ready".into(),
        interval_seconds: 10,
        timeout_seconds: 2,
    };
    plugin.validate().unwrap();
}

#[test]
fn embedded_migrations_cannot_claim_a_directory() {
    let storage = fixtures()
        .into_iter()
        .find(|plugin| plugin.id == "fixture-storage")
        .unwrap();
    assert_eq!(storage.migrations[0].engine, MigrationEngine::Embedded);
    assert!(storage.migrations[0].directory.is_none());

    let mut invalid = storage;
    invalid.migrations[0].directory = Some("migrations".into());
    assert!(matches!(
        invalid.validate(),
        Err(ManifestError::InvalidMigrationShape { .. })
    ));
}
