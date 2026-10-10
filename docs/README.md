# xcss 文档总览

当前源码版本与组件入口见 [根 README](../README.md)。
本目录描述当前接口。版本变化时，以源码、Cargo/npm 清单、运行形态、JSON Schema、测试夹具、测试和发布规则为事实源，在同一变更中更新这里。

| 分类 | 文档 | 适合回答的问题 |
|---|---|---|
| 必要 README | [仓库 README](../README.md) | xcss 是什么、当前组件、硬边界、验证与消费入口 |
| 初学者学习指南 | [十一章教程](beginner-guide/README.md) | 如何阅读认证、合同、SQLite、Web、发布和测试源码 |
| 服务端与 Web 构建 | [统一构建与开发热更新](beginner-guide/11-embedded-web-build.md) | 正式内嵌资源、开发目录、构建顺序与实际产物验收 |
| 工作流程与流程树 | [project-workflow.md](project-workflow.md) | 一个需求怎样进入 xcss、怎样跨产品落地、如何删除或发布 |
| unsafe 审查 | [unsafe-audit.md](unsafe-audit.md) | 原生接口必要性、安全前提、检查与未验证平台 |
| 架构决策 | [architecture/README.md](architecture/README.md) | 平台所有权、运行形态、升级和依赖方向为何如此定义 |
| 平台规范 | [platform-specifications](platform-specifications/platform-migration-roadmap.md) | 当前平台能力与运行形态与能力的正式边界 |
| 服务运行时 | [server-runtime.md](platform-specifications/server-runtime.md) | 统一启停、任务监督与健康检查的 HTTP 边界 |
| 持久操作 | [durable-operations.md](platform-specifications/durable-operations.md) | 事务、所有者隔离、不确定状态（Unknown）和审计发件箱 |
| 文件句柄安全 | [filesystem-handles.md](filesystem-handles.md) | 私有目录、具有类型约束的条目、有界 I/O、原子发布与原生验收边界 |
| 配置、CLI、锁与日志 | [configuration-cli-logging.md](configuration-cli-logging.md) | 当前严格配置、只读诊断、共同维护权与有界结构化日志及其验收边界 |
| 管理员 Web 运行形态 | [admin-web-shell.md](platform-specifications/admin-web-shell.md) | 共享外壳、顶部导航、可访问 UI 和浏览器验收 |
| 管理 Web 中英文 | [admin-web-language.md](admin-web-language.md) | 语言偏好、成对文案、协议值边界和双语验收 |
| 持久管理员管理 | [administrator-management.md](platform-specifications/administrator-management.md) | 唯一管理 API、事务内授权、最后管理员保护、审计与右上角自助设置 |
| 完整功能与取舍清单 | [feature-inventory-and-tradeoffs.md](feature-inventory-and-tradeoffs.md) | 每项能力的实现锚点、分类、复杂度、删除后果、验证和明确排除项 |
| 运维文档 | [operations.md](operations.md) | 固定工具链、CI、软件包与发行运维、故障处置、产品来源复核和安全事件 |

阅读建议：初次参与先读仓库 README 和教程第 1～5 章；设计公共 API 时同时读工作流程与功能清单；准备
标签、依赖更新或事故响应时以运维文档为准。任何文档示例若与当前公开导出入口不一致，应视为发布阻断。

浏览器 WebSocket 使用 `require_administrator_websocket_origin` 验证完整 Origin/Host 列表；Fetch Metadata 可缺失，携带时仍必须是唯一的 `same-origin` 值。普通 HTTP 的校验入口继续要求该字段。应用认证与授权不由此握手校验替代。

## 接入与发布约束

xcss `1.0.0` 不拥有具体产品或业务协议。产品统一依赖一个 Rust crate 和一个 npm 包，按内部模块和公开子路径组合能力。Rust 必须同时固定精确版本与完整 Git 修订；Web 只使用同版不可变发行归档，并在锁文件中保留完整性摘要。

产品根目录维护 `xcss-product.toml`；在 xcss 仓库运行来源检查：

```sh
python3 scripts/check-xcss.py
python3 scripts/xcss-conformance.py report \
  --product-root /absolute/path/to/product --json
```

`report` 是源码接入报告，只有找到并验证发布清单时，`release_verified` 才为 `true`。发布门禁须显式执行 `verify-release --require-published`，不能把缺少发布清单当作产物已验证。正式发行使用 `v1.0.0` 标签；消费者采用前须核对实际 CI、发布清单及不可变资产，既有标签与资产不覆盖。

`xcss::web_assets` 统一生成可内嵌资源清单、SHA-256 身份和 Rust 资源表，提供 GET/HEAD、条件请求与显式开发目录模式。生产资源随可执行文件编译，开发目录模式可热更新，详见 [Web 资源 API](../src/web_assets/README.md)。[日期范围控件](web-date-range.md)支持独立编辑年月日、回车应用和非法数字标红；日志权限、服务器时区边界与范围查询由产品后端校验。

[1.0.0 发布说明](releases/1.0.0.md)与[账号设置](web-account-settings.md)记录账号页和登录页主题、浅色黑字、图标尺寸、窄屏长错误布局及显示前完整字体加载要求。

## 开发验证

固定工具链为 Rust `1.99.0`、Node `26.7.0`、pnpm `10.34.6`。Rust crate 与 npm 构建输入只支持 Linux AMD64 GNU；浏览器客户端不受此限制。

```sh
python3 scripts/check-xcss.py
cargo +1.99.0 fmt --all -- --check
cargo +1.99.0 test --locked --all-targets --all-features
cargo +1.99.0 clippy --locked --all-targets --all-features -- -D warnings
pnpm install --frozen-lockfile --ignore-scripts
pnpm test
```

代码采用 [Apache License 2.0](../LICENSE)。
