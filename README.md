# Sarmg Foundation Server

Sarmg Foundation Server `0.8.9` 为 Sarmg 的 Rust/Axum 服务和管理 Web 提供共享基础能力，包括管理员认证、SQLite 状态、Schema 身份、运行时生命周期、文件系统安全、秘密封装、统一错误合同、设计令牌和 React 管理组件。

本仓库不包含任何具体产品，也不拥有产品业务协议。产品选择需要的 crate/npm 包并在自己的仓库中组合、配置和验收；Client 侧基础能力位于独立的 [sarmg-foundation-client](https://github.com/isarmg/sarmg-foundation-client)。

## 消费方式

Rust 消费者应同时固定版本和完整 Git revision：

```toml
[dependencies]
sarmg-admin-core = { git = "https://github.com/isarmg/sarmg-foundation-server.git", rev = "<full-commit-sha>", version = "=0.8.9" }
```

Web 包使用仓库生成的不可变发行 tarball，并在产品锁文件中保留完整 integrity。产品还需维护 `sarmg-product.toml`，由统一检查脚本核对 Profile、能力、Schema 和依赖身份：

```sh
python3 scripts/check-foundation.py
python3 scripts/check-foundation.py --product-root /absolute/path/to/product
```

正式 Server 运行目标为 Linux AMD64 GNU。完整组件清单、Profile 规则和发布流程见文档总览。

当前发布基线：

| 项目 | 版本 |
|---|---|
| Foundation 版本 | `0.8.9` |
| Rust | `1.98.0` |
| Node / pnpm | `26.7.0` / `10.12.1` |

正式发布对应不可变的 `v0.8.9` tag；版本更新时必须同步源码清单、锁文件和消费者验证记录。

## 开发验证

```sh
python3 scripts/check-foundation.py
cargo +1.98.0 fmt --all -- --check
cargo +1.98.0 test --locked --workspace --all-targets --all-features
cargo +1.98.0 clippy --locked --workspace --all-targets --all-features -- -D warnings
corepack pnpm install --frozen-lockfile --ignore-scripts
corepack pnpm test
```

## 文档

- [文档总览](docs/README.md)
- [初学者指南](docs/beginner-guide/README.md)
- [项目工作流程](docs/project-workflow.md)
- [功能范围与取舍](docs/feature-inventory-and-tradeoffs.md)
- [发布与消费者运维](docs/operations.md)

代码采用 [Apache License 2.0](LICENSE)。
