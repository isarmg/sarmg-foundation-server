# xcss

xcss `1.0.0` 为服务端产品的 Rust/HTTP 服务和管理 Web 提供共享基础能力，包括管理员认证、SQLite 状态、结构身份、运行时生命周期、文件系统安全、秘密封装、统一错误合同、设计令牌和 React 管理组件。

本仓库不包含任何具体产品，也不拥有产品业务协议。服务端产品统一依赖一个 Rust crate `xcss` 和一个 npm 包 `@xcss/web`，通过内部模块及公开子路径组合、配置和验收。

1.0.0 统一普通账号设置页与登录页主题、浅色黑字及图标尺寸，并修复窄屏长错误提示撑宽账号表单的问题。输入字段与报错区域分别布局，字体仍在页面显示前完整加载。详见[版本说明](docs/releases/1.0.0.md)和[账号设置](docs/web-account-settings.md)。

## 消费方式

Rust 消费者应同时固定版本和完整 Git 修订号：

```toml
[dependencies]
xcss = { git = "https://github.com/isarmg/xcss.git", rev = "<full-commit-sha>", version = "=1.0.0" }
```

Web 只安装 `https://github.com/isarmg/xcss/releases/download/v1.0.0/xcss-web-1.0.0.tgz` 一个不可变发行归档；例如从 `@xcss/web/admin-shell`、`@xcss/web/contracts` 导入公开子路径，并在产品锁文件中保留完整性摘要。产品还需维护 `xcss-product.toml`，由统一检查脚本核对运行形态、能力、结构和依赖身份：

```sh
python3 scripts/check-xcss.py
python3 scripts/xcss-conformance.py report \
  --product-root /absolute/path/to/product \
  --json
```

`report` 是源码接入报告；顶层 `release_verified` 只有在实际找到并验证发布清单时才为 `true`。发布门禁应
显式运行 `verify-release --require-published`，不能把“源码树中没有发布清单”解释为产物已验证。

xcss 整个 Rust crate 只能为 Linux AMD64 GNU 编译，npm 构建输入也只支持 Linux x64/glibc。浏览器访问服务端提供的管理 UI 不受客户端操作系统限制。完整组件清单、运行形态规则和发布流程见文档总览。

`xcss::web_assets` 统一生成可内嵌的资源清单、SHA-256 身份与 Rust 资源表，并提供 GET/HEAD、条件请求和开发目录资源服务。生产资源与可执行文件一起编译；开发目录模式显式选择，支持前端热更新。API 和缓存规则见 [crate 文档](src/web_assets/README.md)。

管理 UI 提供[可编辑日期范围控件](docs/web-date-range.md)：独立编辑年月日数字，回车应用，非法日期只标红数字。日志权限、服务器时区边界和范围查询由产品后端校验。

当前发行输入：

| 项目 | 版本 |
|---|---|
| xcss 版本 | `1.0.0` |
| Rust | `1.99.0` |
| Node / pnpm | `26.7.0` / `10.34.6` |

正式发行使用 `v1.0.0` 标签；只有实际 CI、发布清单及不可变资产检查完成后，才将消费者固定到本版完整源码修订号与真实发行归档。既有标签和资产不变。

## 开发验证

```sh
python3 scripts/check-xcss.py
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 test --locked --all-targets --all-features
cargo +1.99.0 clippy --locked --all-targets --all-features -- -D warnings
pnpm install --frozen-lockfile --ignore-scripts
pnpm test
```

## 文档

- [文档总览](docs/README.md)
- [初学者指南](docs/beginner-guide/README.md)
- [项目工作流程](docs/project-workflow.md)
- [功能范围与取舍](docs/feature-inventory-and-tradeoffs.md)
- [发布与消费者运维](docs/operations.md)

代码采用 [Apache License 2.0](LICENSE)。

当前发布版本：**1.0.0**。参见 [1.0.0 发布说明](docs/releases/1.0.0.md)。
