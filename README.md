# Sarmg Platform

这是 Union 的产品中立模块契约与薄基础设施层，不是业务代码集合，也不是可独立部署的
服务。五个业务模块都由 Cargo feature 在编译期选入同一个 Union 发行版，运行时则是由
Union 监督的私有进程；只有 Union 监听公共入口和发布 Release。

平台包含四个 crate：

- `sarmg-platform-core`：可嵌入发行版的模块清单、固定进程拓扑、数据库所有权和健康状态。
- `sarmg-platform-axum`：组合 Union 内的静态网关 adapter，不合并 worker 业务 Router。
- `sarmg-platform-gateway`：worker 对 `gateway-v1` 进程级 token、audience 和 prefix 的校验。
- `sarmg-platform-postgres`：模块自有 migration、search path 和 readiness 的薄支持层。

`modules/` 中的五个 JSON 是机器可读事实源，并由
[`contracts/module-v1.schema.json`](contracts/module-v1.schema.json) 和 Rust `ModuleCatalog`
双重约束。清单不能声明管理员提供的 upstream URL、动态插件路径或公网监听地址。

| 模块 | 私有监听 | Union 网关 | 安装文件 | 数据库 |
|---|---|---|---|---|
| Sentinel | `127.0.0.1:18101` | `/modules/sentinel-monitor` | `sentinel-monitor` | 独立 PostgreSQL database/role |
| Photo Backup | `127.0.0.1:18102` | `/modules/photo-backup` | `photo-backup` | 独立 PostgreSQL database/role |
| Dufs | `127.0.0.1:18103` | `/modules/dufs` | `dufs` | 与共享根同故障域的 SQLite |
| Sunshine | `127.0.0.1:18104` | `/modules/sunshine` | `sunshine` | `sunshine` PostgreSQL schema/独占 role |
| 主机监控 | `127.0.0.1:18105` | `/modules/host-monitoring` | `host-monitoring` | `host_monitoring` PostgreSQL schema/独占 role |

详细边界：

- [平台与进程架构](docs/ARCHITECTURE.md)
- [数据库所有权](docs/DATABASE-OWNERSHIP.md)
- [迁移与验收门禁](docs/COMPILED-PROCESS-MIGRATION.md)

验证：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --locked
```

## 许可证

本仓库第一方代码、文档和模块清单采用 [Apache License 2.0](LICENSE)。
