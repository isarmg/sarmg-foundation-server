# xcss

xcss `1.0.0` 为 Xcss 的 Rust/Axum 服务和管理 Web 提供共享基础能力，包括管理员认证、SQLite 状态、Schema 身份、运行时生命周期、文件系统安全、秘密封装、统一错误合同、设计令牌和 React 管理组件。

本仓库不包含任何具体产品，也不拥有产品业务协议。产品选择需要的 crate/npm 包并在自己的仓库中组合、配置和验收；Client 侧基础能力位于独立的 [xcsc](https://github.com/isarmg/xcsc)。

1.0.0 统一普通账号设置页与登录页主题、浅色黑字及图标尺寸，并修复窄屏长错误提示撑宽账号表单的问题。输入字段与报错区域分别布局，字体仍在页面显示前完整加载。详见[版本说明](docs/releases/1.0.0.md)和[账号设置](docs/web-account-settings.md)。

## 消费方式

Rust 消费者应同时固定版本和完整 Git revision：

```toml
[dependencies]
xcss-admin-core = { git = "https://github.com/isarmg/xcss.git", rev = "<full-commit-sha>", version = "=1.0.0" }
```

Web 包使用仓库生成的不可变发行 tarball，并在产品锁文件中保留完整 integrity。产品还需维护 `xcss-product.toml`，由统一检查脚本核对 Profile、能力、Schema 和依赖身份：

```sh
python3 scripts/check-foundation.py
python3 scripts/xcss-conformance.py report \
  --product-root /absolute/path/to/product \
  --json
```

`report` 是源码接入报告；顶层 `release_verified` 只有在实际找到并验证发布清单时才为 `true`。发布门禁应
显式运行 `verify-release --require-published`，不能把“源码树中没有发布清单”解释为产物已验证。

正式 Server 运行目标为 Linux AMD64 GNU。完整组件清单、Profile 规则和发布流程见文档总览。

`xcss-web-assets` 统一生成可内嵌的资源清单、SHA-256 身份与 Rust 资源表，并提供 GET/HEAD、条件请求和开发目录资源服务。生产资源与可执行文件一起编译；开发目录模式显式选择，支持前端热更新。API 和缓存规则见 [crate 文档](rust/crates/xcss-web-assets/README.md)。

管理 UI 提供[可编辑日期范围控件](docs/web-date-range.md)：独立编辑年月日数字，回车应用，非法日期只标红数字。日志权限、服务器时区边界和范围查询由产品后端校验。

当前发行输入：

| 项目 | 版本 |
|---|---|
| Foundation 版本 | `1.0.0` |
| Rust | `1.99.0` |
| Node / pnpm | `26.7.0` / `10.34.6` |

正式发行使用 `v1.0.0` tag；只有实际 CI、发布清单及不可变资产检查完成后，才将消费者固定到本版完整源码 revision 与真实 tarball。既有 tag 和资产不变。

## 开发验证

```sh
python3 scripts/check-foundation.py
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 test --locked --workspace --all-targets --all-features
cargo +1.99.0 clippy --locked --workspace --all-targets --all-features -- -D warnings
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

当前发布版本：**1.0.0**。参见 [1.0.0 发布说明](docs/releases/1.0.0.md)和[项目命名](docs/naming.md)。
