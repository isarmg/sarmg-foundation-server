# Sarmg Platform

这是混合架构的产品中立契约与 SDK：系统以 Modular Monolith 为基础，以 Plugin Architecture
作为扩展机制，并允许重型模块演进为独立 Process、Container 或 Service。Core Platform 只提供
认证、RBAC、配置、审计、任务、通知、模块生命周期、Gateway、服务发现和事件总线，不包含
Sunshine、主机监控、Sentinel、Photo Backup 或 Dufs 的业务逻辑。

模块选择分为两个不同阶段：Builder 在发行构建阶段决定哪些完整模块包进入发行；Core 在运行时
只发现、校验、注册和启停当前发行已包含的包。系统不从公网下载任意代码，也不是在线插件市场。

## Workspace

- `sarmg-platform-core`：统一 Plugin Manifest、严格解析、安全校验、SemVer compatibility 与依赖拓扑。
- `sarmg-platform-sdk`：配置、认证、审计、日志、任务、通知、服务发现、事件和 in-process 生命周期接口。
- `sarmg-platform-events`：transport-neutral Event Envelope、Publisher、Subscription 与 delivery 语义。
- `sarmg-platform-gateway`：旧 worker `gateway-v1` 身份适配器。
- `sarmg-platform-postgres`：模块自有 PostgreSQL migration/readiness 薄支持层。
- `sarmg-platform-axum`：可选的 Axum host adapter；Core/SDK/Event 均不依赖 Web framework。

机器契约在 [`contracts/plugin-manifest-v1.schema.json`](contracts/plugin-manifest-v1.schema.json)，
Rust 权威语义校验在 `sarmg-platform-core`。五份内置 manifest 是迁移基线，不是编译期 allow-list。
完整可复制包见 [`examples/process-module`](examples/process-module)。

## 核心保证

- Manifest 每层对象拒绝未知字段；所有版本和 compatibility range 使用 SemVer。
- dependency 必须存在且版本匹配，激活按确定性拓扑顺序进行；循环依赖拒绝。
- 可执行文件、前端、配置、migration 与 release notes 均为安全 bundle 相对路径，禁止 traversal。
- canonical API 固定 `/api/modules/<id>`；每条 route 显式声明安全 `upstream_path` 和认证边界。
- `auth=platform` 必须引用模块已声明权限；`auth=module` 必须 `permission=null`，但仍只能经 Core 公共 Gateway。
- 前端资源固定 `/modules/<id>/assets/<relative>`；route 固定 `/modules/<id>` 命名空间。
- process 仅 loopback，配置由只读 JSON 文件传入；旧 env 只能通过 `config_pointer` 显式映射且不经 shell。
- PostgreSQL schema/migration 属于单一模块；禁止跨模块 SQL 和直接修改其他模块数据。

## 版本

平台 crate 当前为 `0.2.0`。`platform_api=1.0.0` 与 `plugin_api=1.0.0` 是独立兼容面，不能从
crate 版本推导。当前 Union Core 插件兼容窗口为 `>=0.5.0, <0.6.0`。

## 文档与验证

- [混合架构与边界](docs/ARCHITECTURE.md)
- [Plugin Manifest / SDK / Event 契约](docs/PLUGIN-CONTRACT.md)
- [运行期插件迁移与验收](docs/COMPILED-PROCESS-MIGRATION.md)
- [数据库所有权](docs/DATABASE-OWNERSHIP.md)

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

本仓库第一方代码、文档与示例采用 [Apache License 2.0](LICENSE)。
