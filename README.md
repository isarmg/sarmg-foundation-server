# Sarmg Foundation

Sarmg Foundation `0.2.0` 提供少量经过验证、仅在构建期复用的 Rust 与 Web 基础组件。它不是运行时
服务：业务产品把选定 crate/package 编译进自己的不可变制品，并继续独立拥有用户、Session、数据库、
文件、进程和发布周期。

所有包仍为实验性 `0.x`。新版本不会保留旧 API alias 或 re-export；消费者必须显式修改并在自身仓库
完成集成测试。

## 当前组件

```text
rust/crates/sarmg-error    严格 ErrorCode、ErrorEnvelope 与 HTTP 状态映射
rust/crates/sarmg-sqlite   SQLx SQLite pool、完整性/外键检查与 WAL checkpoint
packages/contracts         Error、State、Release、Backup 等 wire contract/JSON Schema
packages/http-client       有界 same-origin JSON fetch client
packages/design-tokens     TypeScript token 与明/暗色 CSS
```

## 快速验证

```bash
cargo +1.98.0 fmt --all -- --check
cargo +1.98.0 check --workspace --all-targets
cargo +1.98.0 clippy --workspace --all-targets -- -D warnings
cargo +1.98.0 test --workspace
pnpm install --frozen-lockfile
pnpm typecheck
pnpm build
pnpm test
```

## 文档

- [文档总览](docs/README.md)
- [初学者学习指南](docs/beginner-guide/README.md)
- [项目工作流程与流程树](docs/project-workflow.md)
- [完整功能与取舍清单](docs/feature-inventory-and-tradeoffs.md)
- [版本、依赖、发布与故障运维](docs/operations.md)

代码采用 [Apache License 2.0](LICENSE)。
