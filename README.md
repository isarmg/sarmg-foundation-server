# Sarmg Platform

这是从 UnionC 提炼出的运行平台层，不是业务代码集合，也不替代
[`../upstream`](../upstream/README.md) 中的设计、HTTP 和安全契约。

平台提供三项能力：

- `sarmg-platform-core`：框架无关的模块描述、数据库所有权和运行状态契约。
- `sarmg-platform-axum`：编译期 Axum 模块的路由组装；不使用 Rust 动态链接插件。
- `sarmg-platform-postgres`：PostgreSQL 连接、migration 和 readiness 的薄支持层。

`modules/` 是五个首批模块的机器可读清单。Sunshine 和主机监控是进程内模块；
Sentinel、Photo Backup 和 Dufs 是独立服务模块。独立服务仍保留自己的认证、发布周期、
业务数据库和故障边界，平台只接入导航、健康状态和后续的受控身份交换。

验证：

```bash
cargo fmt --all -- --check
cargo test --workspace
```

正式发布前应把本目录初始化为独立仓库并发布固定版本；当前 sibling path 依赖只用于
`/mnt/sarmg.org` 组合工作区内的迁移。

