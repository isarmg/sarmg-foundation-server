# xcss 文档总览

当前源码版本与组件入口见 [根 README](../README.md)。
[0.7.1 消费者证据](../consumers/react-filesystem-0.7.1-evidence.md)仅记录历史验收，不代表当前版本。主分支后续文档修订不改变已发布资产，既有不可变 tag 不会被改写。
本目录描述当前接口。版本变化时，以源码、Cargo/npm manifest、Profile、JSON Schema、fixture、测试和发布 policy 为事实源，在同一变更中更新这里。

| 分类 | 文档 | 适合回答的问题 |
|---|---|---|
| 必要 README | [仓库 README](../README.md) | xcss 是什么、当前组件、硬边界、验证与消费入口 |
| 初学者学习指南 | [十一章教程](beginner-guide/README.md) | 如何阅读认证、合同、SQLite、Web、发布和测试源码 |
| Server/Web 构建 | [统一构建与开发热更新](beginner-guide/11-embedded-web-build.md) | 正式内嵌资源、开发目录、构建顺序与实际产物验收 |
| 工作流程与流程树 | [project-workflow.md](project-workflow.md) | 一个需求怎样进入 xcss、怎样跨产品落地、如何删除或发布 |
| unsafe 审查 | [unsafe-audit.md](unsafe-audit.md) | 原生接口必要性、安全前提、检查与未验证平台 |
| 架构决策 | [architecture/README.md](architecture/README.md) | 平台所有权、Profile、升级和依赖方向为何如此定义 |
| 平台规范 | [platform-specifications](platform-specifications/platform-migration-roadmap.md) | 当前平台能力与 Profile/Capability 的正式边界 |
| Server Runtime | [server-runtime.md](platform-specifications/server-runtime.md) | 统一启停、任务监督与健康检查的 HTTP 边界 |
| Durable Operations | [durable-operations.md](platform-specifications/durable-operations.md) | 事务、owner fencing、Unknown 和审计 outbox |
| 文件句柄安全 | [filesystem-handles.md](filesystem-handles.md) | 私有目录、typed entry、有界 I/O、原子发布与原生验收边界 |
| 配置、CLI、锁与日志 | [configuration-cli-logging.md](configuration-cli-logging.md) | 当前工作树新增的严格配置、只读诊断、共同维护权与有界结构化日志；未发布边界 |
| 管理员 Web Profile | [admin-web-shell.md](platform-specifications/admin-web-shell.md) | 共享外壳、顶部导航、可访问 UI 和浏览器验收 |
| 管理 Web 中英文 | [admin-web-language.md](admin-web-language.md) | 语言偏好、成对文案、协议值边界和双语验收 |
| 持久管理员管理 | [administrator-management.md](platform-specifications/administrator-management.md) | 唯一管理 API、事务内授权、最后管理员保护、审计与右上角自助设置 |
| 完整功能与取舍清单 | [feature-inventory-and-tradeoffs.md](feature-inventory-and-tradeoffs.md) | 每项能力的实现锚点、分类、复杂度、删除后果、验证和明确排除项 |
| 运维文档 | [operations.md](operations.md) | 固定工具链、CI/package/release 运维、故障处置、消费者追踪和安全事件 |

阅读建议：初次参与先读仓库 README 和教程第 1～5 章；设计公共 API 时同时读工作流程与功能清单；准备
tag、依赖更新或事故响应时以运维文档为准。任何文档示例若与当前 public export 不一致，应视为发布阻断。

浏览器 WebSocket 使用 `require_administrator_websocket_origin` 验证完整 Origin/Host 列表；Fetch Metadata 可缺失，携带时仍必须单一 same-origin。普通 HTTP 的校验入口继续要求该字段。应用认证与授权不由此握手校验替代。
