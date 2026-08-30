# isarmg-foundation

Experimental, build-time shared primitives for the independent ISArmg products:

```text
photo-backup
host-monitoring
dufs-ram
sentinel-monitor
sunshine-manager
```

The foundation is not a runtime service. Products compile selected crates and packages into their
own release artifacts and keep their own users, sessions, databases, files and processes. A built
product must continue to run when this repository or its package registries are unavailable.

## Stability

Every package in this repository is currently `0.x` and experimental. None of the packages is a
complete production security boundary yet:

- `isarmg-auth` provides password hashing and a signed, stateless token primitive; it does not
  provide persistent sessions, revocation, idle expiry, session-bound CSRF verification or login
  admission control.
- `isarmg-sqlite` provides an async SQLx pool baseline, migration runner, integrity/foreign-key
  checks and WAL checkpointing while retaining its legacy synchronous API. It does not yet provide
  online backups, restore validation, permission enforcement or operational metrics.
- `isarmg-path-validation` performs lexical relative-path validation only. It does not provide an
  FD-anchored filesystem root or protect callers from symlink and TOCTOU attacks.
- `isarmg-operations` contains state data types only. It does not provide persistence, leases,
  idempotency, retries, an outbox or crash recovery.
- `isarmg-error` and `@isarmg/contracts` share a validated, machine-readable `ErrorEnvelope`
  wire shape. They remain 0.x: product-specific codes and adoption still require compatibility
  tests in each consumer.
- `@isarmg/http-client` now ships compiled output with bounded JSON reads, same-origin credentials,
  timeouts, CSRF propagation and typed errors. It is still experimental until adopted and tested
  by at least two products.
- The Web packages are source-level prototypes unless their own package metadata explicitly
  declares a build and distributable `dist` output.

Business products must keep their existing stronger local implementations until a Foundation
replacement has equivalent behavior, tests and at least two real consumers.

## Rust crates

```text
rust/crates/
├── isarmg-error
├── isarmg-http
├── isarmg-auth
├── isarmg-sqlite
├── isarmg-observability
├── isarmg-config
├── isarmg-path-validation
└── isarmg-operations
```

## Web packages

```text
web/packages/
├── design-tokens
├── ui
├── app-shell
├── http-client
├── web-config
├── contracts
├── auth-ui
└── testkit
```

When a package becomes publishable, business projects should depend on an exact released version.
Already built products must never load Foundation code from a shared runtime service or CDN.

## Development

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
