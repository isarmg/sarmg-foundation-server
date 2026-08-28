# Contract test fixtures

These files are synthetic inputs for Platform contract tests. They intentionally do not describe
any Union business module and must not be copied into a release.

Each module repository owns its `manifest.json`. Union Builder reads and validates that file from
the selected source revision when assembling a release; Platform provides only the schema and Rust
semantic validator.
