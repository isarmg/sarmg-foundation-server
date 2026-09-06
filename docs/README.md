# Sarmg Foundation 文档总览

本工作区包含 `0.5.0` 之后尚未发布的平台化改造；既有不可变 tag 不代表这些新增实现，也不会被改写。
本目录描述唯一当前接口，不保存旧版兼容说明。发生版本变化时，以源码、Cargo/npm
manifest、Profile、JSON Schema、fixture、测试和发布 policy 为事实源，在同一变更中更新这里。

| 分类 | 文档 | 适合回答的问题 |
|---|---|---|
| 必要 README | [仓库 README](../README.md) | Foundation 是什么、当前组件、硬边界、验证与消费入口 |
| 初学者学习指南 | [十章教程](beginner-guide/README.md) | 如何阅读认证、合同、SQLite、Web、发布和测试源码 |
| 工作流程与流程树 | [project-workflow.md](project-workflow.md) | 一个需求怎样进入 Foundation、怎样跨产品落地、如何删除或发布 |
| 架构决策 | [architecture/README.md](architecture/README.md) | 平台所有权、Profile、升级和依赖方向为何如此定义 |
| 平台规范 | [platform-specifications](platform-specifications/platform-migration-roadmap.md) | 平台迁移阶段与 Profile/Capability 的正式边界 |
| Server Runtime | [server-runtime.md](platform-specifications/server-runtime.md) | 统一启停、任务监督与健康检查；诊断 HTTP 接口已移除 |
| Durable Operations | [durable-operations.md](platform-specifications/durable-operations.md) | 事务、owner fencing、Unknown 和审计 outbox |
| 文件句柄安全 | [filesystem-handles.md](filesystem-handles.md) | 私有目录、typed entry、有界 I/O、原子发布与原生验收边界 |
| 管理员 Web Profile | [admin-web-shell.md](platform-specifications/admin-web-shell.md) | 共享外壳、顶部导航、可访问 UI 和浏览器验收 |
| 持久管理员管理 | [administrator-management.md](platform-specifications/administrator-management.md) | 唯一管理 API、事务内授权、最后管理员保护、审计与共享面板 |
| 完整功能与取舍清单 | [feature-inventory-and-tradeoffs.md](feature-inventory-and-tradeoffs.md) | 每项能力的实现锚点、分类、复杂度、删除后果、验证和明确排除项 |
| 运维文档 | [operations.md](operations.md) | 固定工具链、CI/package/release 运维、故障处置、消费者追踪和安全事件 |

四个 package 目录中的 `README.md` 是发布包随附的必要 README，仍属于“必要 README”分类；它们只解释
各自公开入口和边界，不另建教程体系。Foundation 无生产 daemon，所以没有启动、systemd、业务数据备份
或在线告警 runbook；相关工作分别属于各产品及 `sarmg-upgrade`。

阅读建议：初次参与先读仓库 README 和教程第 1～5 章；设计公共 API 时同时读工作流程与功能清单；准备
tag、依赖更新或事故响应时以运维文档为准。任何文档示例若与当前 public export 不一致，应视为发布阻断。
