# Sarmg Platform

这是从 UnionC 提炼出的运行平台层，不是业务代码集合，也不替代
[`../upstream`](../upstream/README.md) 中的设计、HTTP 和安全契约。

平台提供三项能力：

- `sarmg-platform-core`：框架无关的模块描述、数据库所有权和运行状态契约。
- `sarmg-platform-axum`：编译期 Axum 模块的路由组装；不使用 Rust 动态链接插件。
- `sarmg-platform-postgres`：PostgreSQL 连接、migration 和 readiness 的薄支持层。

`modules/` 是五个首批模块的机器可读清单。当前代码仍处于过渡态：Sunshine/主机监控在
Union 进程内，Sentinel/Photo/Dufs 通过运行时 URL 接入。目标架构已改为全部模块在构建期
选入一个 Union 发行版、运行时使用私有独立进程、只由 Union 提供公共入口和 Release。
迁移门禁见 [`docs/COMPILED-PROCESS-MIGRATION.md`](docs/COMPILED-PROCESS-MIGRATION.md)；在门禁
完成前不得把当前 manifest 当成最终模块 ABI。

验证：

```bash
cargo fmt --all -- --check
cargo test --workspace
```

本目录已经是独立仓库；正式发布 crate 前仍应建立固定版本。当前 sibling path 依赖只用于
`/mnt/sarmg.org` 组合工作区内的迁移。

## 许可证

本仓库的第一方代码、文档和模块清单采用 [Apache License 2.0](LICENSE)。
