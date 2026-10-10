# xcss

xcss 是服务端与管理 Web 的公共基础库，提供一个 Rust crate `xcss` 和一个 npm 包 `@xcss/web`，由具体产品集成使用。

## 项目功能

- 管理员认证、统一错误与数据合同、SQLite 状态校验和运行时生命周期。
- 安全文件操作、秘密封装、结构化日志和持久操作。
- React 管理组件、设计令牌、HTTP 客户端及内嵌 Web 资源构建。

## 适用平台

Rust 与 Web 构建仅支持 Linux x86_64 GNU（glibc）。部署后的管理页面可通过不同操作系统上的浏览器访问。

## 快速部署

本项目通过依赖接入，不单独运行服务。以下示例使用已发布的 1.0.2。Rust 消费者在 `Cargo.toml` 固定精确版本与完整发布修订：

```toml
[dependencies]
xcss = { git = "https://github.com/isarmg/xcss.git", rev = "3f751196615edd9f7fda2d76a5aa90f9f42586dc", version = "=1.0.2" }
```

需要管理 Web 的产品安装同版发行包，并提交依赖锁文件：

```sh
npm install --save-exact https://github.com/isarmg/xcss/releases/download/v1.0.2/xcss-web-1.0.2.tgz
```

通过 `xcss::<module>`、`@xcss/web/<module>` 引入能力，在产品根目录维护 `xcss-product.toml`；React/Vite 等 peer 依赖按包内精确版本安装。完成产品构建后，随该产品部署。

## 编译部署

准备 Rust `1.99.0`、Node `26.7.0`、pnpm `10.34.6`、Python 3 和 C 编译工具链，在仓库根目录执行（`release/npm/` 须为空或尚未创建）：

```sh
python3 scripts/check-xcss.py
cargo build --release --locked
pnpm install --frozen-lockfile --ignore-scripts
python3 scripts/package-artifacts.py release --output release/npm
```

Rust 构建产物位于 `target/release/`，Web 归档位于 `release/npm/`。本地归档用于接入验证；正式消费者使用已发布的固定 Git 修订和发行包，重新编译并部署各自产品。

[详细文档](docs/README.md)
