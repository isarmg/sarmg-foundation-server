# isarmg-foundation

Versioned, build-time shared foundation for the independent ISArmg products:

```text
photo-backup
host-monitoring
dufs-ram
sentinel-monitor
sunshine-manager
```

The foundation is not a runtime service. Products compile these crates and packages into their
own release artifacts and keep their own users, sessions, databases, files and processes.

## Rust crates

```text
rust/crates/
├── isarmg-error
├── isarmg-http
├── isarmg-auth
├── isarmg-sqlite
├── isarmg-observability
├── isarmg-config
├── isarmg-rooted-storage
├── isarmg-operations
└── isarmg-postgres
```

Business projects should depend on pinned, published versions. During local development the
workspace may use path dependencies; release CI must replace them with registry versions.

## Development

```bash
cargo check --workspace --all-targets
cargo test --workspace
```
