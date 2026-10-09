use super::*;
use serde::Deserialize;

#[derive(Debug)]
struct ValidatedSecret;
impl<'de> Deserialize<'de> for ValidatedSecret {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        if value == "valid" {
            Ok(Self)
        } else {
            Err(serde::de::Error::custom("sensitive validator diagnostic"))
        }
    }
}

#[test]
fn custom_deserializers_keep_nested_object_and_array_paths_without_values() {
    #[derive(Debug, Deserialize)]
    struct Nested {
        _items: Vec<ValidatedSecret>,
    }
    // Rename lets the test exercise JSON-pointer escaping in an actual field.
    #[derive(Debug, Deserialize)]
    struct Config {
        #[serde(rename = "a/b~c")]
        nested: Nested,
    }
    for (value, expected) in [
        (
            json!({"a/b~c":{"_items":["valid","private-input"]}}),
            "/a~1b~0c/_items/1",
        ),
        (json!({"a/b~c":{"_items":[1]}}), "/a~1b~0c/_items/0"),
    ] {
        let error = strict::deserialize::<Config>(value, ConfigSource::File).unwrap_err();
        assert_eq!(error.path, expected);
        assert_eq!(error.source, ConfigSource::File);
        let rendered = serde_json::to_string(&error.envelope()).unwrap();
        assert!(!rendered.contains("private-input"));
        assert!(!rendered.contains("sensitive validator diagnostic"));
    }
    let valid =
        strict::deserialize::<Config>(json!({"a/b~c":{"_items":["valid"]}}), ConfigSource::File)
            .unwrap();
    assert_eq!(valid.nested._items.len(), 1);
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Config {
    listen: String,
    workers: u16,
    tls: Tls,
    targets: Vec<String>,
    #[serde(default)]
    optional: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Tls {
    enabled: bool,
    password: String,
}

fn defaults() -> Config {
    Config {
        listen: "127.0.0.1:8000".into(),
        workers: 4,
        tls: Tls {
            enabled: true,
            password: "default-secret".into(),
        },
        targets: vec![],
        optional: None,
    }
}

#[test]
fn explicit_sources_have_one_precedence_and_nested_leaf_provenance() {
    let env = read_environment(
        &[
            EnvMapping {
                variable: "APP_LISTEN",
                path: "/listen",
                kind: EnvValueKind::String,
            },
            EnvMapping {
                variable: "APP_WORKERS",
                path: "/workers",
                kind: EnvValueKind::UnsignedInteger,
            },
        ],
        |key| match key {
            "APP_LISTEN" => Some("127.0.0.1:9000".into()),
            "APP_WORKERS" => Some("12".into()),
            _ => panic!("undeclared environment name read"),
        },
    )
    .unwrap();
    let file = br#"{"listen":"127.0.0.1:8080","workers":8,"tls":{"password":"file-secret"}}"#;
    let result = resolve(
        &defaults(),
        Some(file),
        &env,
        &[Override::new("/listen", "127.0.0.1:10000")],
    )
    .unwrap();
    assert_eq!(result.value.listen, "127.0.0.1:10000");
    assert_eq!(result.value.workers, 12);
    assert!(result.value.tls.enabled);
    assert_eq!(result.value.tls.password, "file-secret");
    assert_eq!(result.sources["/listen"], ConfigSource::CommandLine);
    assert_eq!(result.sources["/workers"], ConfigSource::Environment);
    assert_eq!(result.sources["/tls/password"], ConfigSource::File);
    assert_eq!(result.sources["/tls/enabled"], ConfigSource::Default);
    let diagnostic = serde_json::to_string(&result.sources).unwrap();
    assert!(!diagnostic.contains("secret"));
}

#[test]
fn malformed_lower_priority_input_cannot_be_hidden_by_an_override() {
    let error = resolve(
        &defaults(),
        Some(br#"{"workers":"secret-number"}"#),
        &[],
        &[Override::new("/workers", 2)],
    )
    .err()
    .unwrap();
    assert_eq!(error.reason, Reason::TypeMismatch);
    assert_eq!(error.path, "/workers");
    assert_eq!(error.source, ConfigSource::File);
    assert!(
        !serde_json::to_string(&error.envelope())
            .unwrap()
            .contains("secret-number")
    );
}

#[test]
fn product_value_constraints_are_checked_before_higher_priority_overrides() {
    let validate = |config: &Config, source| {
        if config.workers == 0 {
            Err(ConfigError::new(Reason::InvalidValue, "/workers", source))
        } else {
            Ok(())
        }
    };
    let result = resolve_validated(
        &defaults(),
        Some(br#"{"workers":0}"#),
        &[],
        &[Override::new("/workers", 2)],
        validate,
    );
    let error = result.err().unwrap();
    assert_eq!(
        (error.reason, error.path.as_str(), error.source),
        (Reason::InvalidValue, "/workers", ConfigSource::File)
    );
}

#[test]
fn rejects_unknown_fields_without_requiring_a_serde_annotation() {
    for (bytes, path) in [
        (&br#"{"old_listen":"secret"}"#[..], "/old_listen"),
        (&br#"{"tls":{"unknown":"secret"}}"#[..], "/tls/unknown"),
    ] {
        let error = resolve(&defaults(), Some(bytes), &[], &[]).err().unwrap();
        assert_eq!(error.reason, Reason::UnknownField);
        assert_eq!(error.path, path);
    }
}

#[test]
fn rejects_duplicate_fields_invalid_arrays_missing_fields_and_null() {
    for (bytes, reason, path) in [
        (
            &br#"{"tls":{"password":"a","password":"b"}}"#[..],
            Reason::DuplicateField,
            "/tls/password",
        ),
        (
            &br#"{"targets":["valid",9]}"#[..],
            Reason::TypeMismatch,
            "/targets/1",
        ),
        (&br#"{"tls":null}"#[..], Reason::TypeMismatch, "/tls"),
        (
            &br#"{"tls":[true,"array-secret"]}"#[..],
            Reason::TypeMismatch,
            "/tls",
        ),
        (&br#"[]"#[..], Reason::TypeMismatch, ""),
    ] {
        let error = resolve(&defaults(), Some(bytes), &[], &[]).err().unwrap();
        assert_eq!((error.reason, error.path.as_str()), (reason, path));
    }
    let error = resolve(
        &defaults(),
        None,
        &[],
        &[Override::new("/tls", json!({"enabled":true}))],
    )
    .err()
    .unwrap();
    assert_eq!(
        (error.reason, error.path.as_str()),
        (Reason::MissingField, "/tls/password")
    );
}

#[test]
fn environment_types_are_explicit_and_errors_never_expose_values() {
    for kind in [
        EnvValueKind::Boolean,
        EnvValueKind::UnsignedInteger,
        EnvValueKind::SignedInteger,
        EnvValueKind::Json,
    ] {
        let error = read_environment(
            &[EnvMapping {
                variable: "APP_INPUT",
                path: "/workers",
                kind,
            }],
            |_| Some("secret-invalid".into()),
        )
        .unwrap_err();
        assert!(!format!("{error:?}").contains("secret-invalid"));
        assert_eq!(error.path, "/workers");
        assert_eq!(error.source, ConfigSource::Environment);
    }
    let error = read_environment(
        &[EnvMapping {
            variable: "APP_TLS",
            path: "/tls",
            kind: EnvValueKind::Json,
        }],
        |_| Some(r#"{"enabled":false,"enabled":true}"#.into()),
    )
    .unwrap_err();
    assert_eq!(
        (error.reason, error.path.as_str()),
        (Reason::DuplicateField, "/tls/enabled")
    );
}

#[test]
fn input_budgets_invalid_pointers_and_ambiguous_overrides_fail() {
    let error = resolve(
        &defaults(),
        Some(&vec![b' '; MAX_CONFIG_BYTES + 1]),
        &[],
        &[],
    )
    .err()
    .unwrap();
    assert_eq!(error.reason, Reason::LimitExceeded);
    for path in ["workers", "/tls/~2", "/targets/0"] {
        assert!(resolve(&defaults(), None, &[], &[Override::new(path, 1)]).is_err());
    }
    let error = resolve(
        &defaults(),
        None,
        &[],
        &[Override::new("/workers", 1), Override::new("/workers", 2)],
    )
    .err()
    .unwrap();
    assert_eq!(error.reason, Reason::DuplicateField);
    assert!(
        !format!("{:?}", Override::new("/tls/password", "debug-secret")).contains("debug-secret")
    );
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
enum Mode {
    Local,
    Remote { host: String },
    Port(u16),
}
#[derive(Debug, Serialize, Deserialize)]
struct EnumConfig {
    mode: Mode,
}

#[test]
fn normal_current_serde_enum_contracts_work_with_safe_value_failures() {
    let default = EnumConfig { mode: Mode::Local };
    for (bytes, expected) in [
        (&br#"{"mode":"Local"}"#[..], Mode::Local),
        (
            &br#"{"mode":{"Remote":{"host":"localhost"}}}"#[..],
            Mode::Remote {
                host: "localhost".into(),
            },
        ),
        (&br#"{"mode":{"Port":80}}"#[..], Mode::Port(80)),
    ] {
        assert_eq!(
            resolve(&default, Some(bytes), &[], &[]).unwrap().value.mode,
            expected
        );
    }
    let error = resolve(&default, Some(br#"{"mode":"enum-secret"}"#), &[], &[])
        .err()
        .unwrap();
    assert_eq!(error.reason, Reason::InvalidValue);
    assert!(!format!("{error:?}").contains("enum-secret"));
}

#[cfg(unix)]
#[test]
fn private_configuration_read_is_bounded_readonly_and_rejects_unsafe_files() {
    use std::{
        fs,
        os::unix::fs::{PermissionsExt, symlink},
    };
    let directory = tempfile::tempdir().unwrap();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700)).unwrap();
    let path = directory.path().join("config.json");
    fs::write(&path, b"{\"workers\":4}").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(read_private_file(&path).unwrap(), b"{\"workers\":4}");
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    let missing = directory.path().join("missing.json");
    assert_eq!(
        read_private_file(&missing)
            .unwrap_err()
            .envelope()
            .code
            .as_str(),
        "config_file_not_found"
    );
    let fifo = directory.path().join("fifo.json");
    rustix::fs::mkfifoat(
        rustix::fs::CWD,
        &fifo,
        rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR,
    )
    .unwrap();
    assert_eq!(
        read_private_file(&fifo).unwrap_err().reason,
        Reason::UnsafeFile
    );
    fs::remove_file(fifo).unwrap();
    let link = directory.path().join("link.json");
    symlink(&path, &link).unwrap();
    assert!(read_private_file(&link).is_err());
    fs::hard_link(&path, directory.path().join("alias.json")).unwrap();
    assert!(read_private_file(&path).is_err());
    fs::remove_file(directory.path().join("alias.json")).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert!(read_private_file(&path).is_err());
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    fs::write(&path, vec![0; MAX_CONFIG_BYTES + 1]).unwrap();
    assert_eq!(
        read_private_file(&path).unwrap_err().reason,
        Reason::LimitExceeded
    );
}
