# xcss 专题参考

首次接入请从[开始接入](../getting-started.md)阅读。以下页面按技术问题查阅。

## Rust 和数据

- [公开模块](modules.md)
- [管理员认证原语](authentication.md)
- [HTTP 正文预算](../platform-specifications/administrator-http-bodies.md)
- [单管理员账户更新](../platform-specifications/administrator-management.md)
- [通信合同与 SQLite](contracts-and-schema.md)
- [文件句柄](../filesystem-handles.md)
- [配置、CLI、状态锁与日志](../configuration-cli-logging.md)
- [服务运行时](../platform-specifications/server-runtime.md)
- [持久操作](../platform-specifications/durable-operations.md)

## 管理 Web

- [HTTP 与认证生命周期](http-client.md)
- [React Shell](../platform-specifications/admin-web-shell.md)与[原生 ESM](../platform-specifications/native-module-web.md)
- [工作区默认值](../admin-workspace.md)、[语言](../admin-web-language.md)、[账户设置](../web-account-settings.md)、[日期范围](../web-date-range.md)
- [设计令牌](design-tokens.md)
- [内嵌 Web 构建](embedded-web.md)与 [Rust 资源 API](../../src/web_assets/README.md)

## 开发和设计

- [软件包和发行格式](package-release.md)
- [专项测试](testing.md)与[安全维护](security-maintenance.md)
- [能力实现索引](capability-inventory.md)、[源码导航和术语](reading-and-glossary.md)
- [架构决策](../architecture/README.md)、[运行形态](../platform-specifications/profiles-and-capabilities.md)、[能力接入](../platform-specifications/platform-migration-roadmap.md)
- [公共库与产品职责](../server-client-repositories.md)
- [当前 unsafe 审查](../unsafe-audit.md)与[历史逐函数记录](../unsafe-review-0.11.0.md)
