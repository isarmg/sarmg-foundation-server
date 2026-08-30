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

Every package in this repository is currently `0.x` and experimental:

- `isarmg-sqlite` provides the current async SQLx pool baseline, integrity/foreign-key checks and
  WAL checkpointing. Products own their schema lifecycle; Foundation does not retain an older
  database API or run product schema changes.
- `isarmg-error` and `@isarmg/contracts` share a validated, machine-readable `ErrorEnvelope`
  wire shape. They remain 0.x: product-specific codes and adoption still require contract
  tests in each consumer.
- `@isarmg/http-client` now ships compiled output with bounded JSON reads, same-origin credentials,
  timeouts, CSRF propagation and typed errors. It is still experimental until adopted and tested
  by at least two products.
- The Web workspace contains only packages with a build and distributable `dist` output.
  Placeholder UI, shell, authentication, testkit and global API-prefix packages were deleted;
  products own those concerns until a tested shared implementation has real consumers.

The former authentication, HTTP middleware, configuration, observability, path-validation and
operation-type crates were also deleted. They had no real consumers and were weaker than the
product-local boundaries. A shared replacement must be designed from current requirements and
prove itself in at least two products; no removed API will be carried forward as compatibility.

Business products must own stronger local implementations until a Foundation replacement has
equivalent behavior, tests and at least two real consumers.

## Rust crates

The Rust workspace is versioned `0.2.0`. Removed `0.1` APIs are not aliased or re-exported.

```text
rust/crates/
├── isarmg-error
└── isarmg-sqlite
```

## Web packages

All Web packages are versioned `0.2.0`. Removed `0.1` names and entry points are not aliased or
re-exported; consumers must update imports explicitly.

```text
web/packages/
├── design-tokens
├── http-client
└── contracts
```

When a package becomes publishable, business projects should depend on an exact released version.
Already built products must never load Foundation code from a shared runtime service or CDN.

## Development

```bash
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
