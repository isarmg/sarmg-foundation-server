# 功能索引

xcss 提供一个 Rust crate 与一个管理 Web npm 包。按需要组合内部模块和公开子路径。

| 任务 | 入口 | 参考 |
|---|---|---|
| 管理员认证与账户更新 | admin_core、admin_sqlite/admin_static、HTTP 适配器 | [认证](reference/authentication.md)、[账户接口](platform-specifications/administrator-management.md) |
| 配置、CLI 与日志 | config、server_cli、log | [配置与日志](configuration-cli-logging.md) |
| 私有文件和状态锁 | fs_safety、state_file | [文件句柄](filesystem-handles.md) |
| SQLite 与数据身份 | sqlite、schema_identity、platform_db | [合同与结构](reference/contracts-and-schema.md) |
| HTTP 服务生命周期 | server_runtime | [运行时](platform-specifications/server-runtime.md) |
| 持久操作和审计 | operations | [操作接口](platform-specifications/durable-operations.md) |
| 管理页面和认证请求 | admin-shell、admin-web、http-client | [Web Shell](platform-specifications/admin-web-shell.md)、[请求](reference/http-client.md) |
| 样式、字体和控件 | admin-ui、design-tokens、web-fonts | [工作区](admin-workspace.md)、[设计令牌](reference/design-tokens.md) |
| 内嵌资源与发行验证 | web_assets、web-toolchain、Python 工具 | [构建](reference/embedded-web.md)、[发行](operations.md) |

产品保留业务协议、数据、页面、设备和部署策略；公共库提供通用机制。共享能力按职责和可测试性划分，具体原则见[架构决策](architecture/README.md)。

逐项查找源码与测试时使用[能力实现索引](reference/capability-inventory.md)。
