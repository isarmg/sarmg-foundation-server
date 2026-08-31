# 10. 源码路线、练习与术语表

## 10.1 阅读路线

先读 workspace/package manifests，掌握公开 export；再读每个组件实现和测试；最后读 CI、构建与消费者
文档。不要从 node_modules 或 target 反推公共 API。

## 10.2 按问题找入口

| 问题 | 入口 |
|---|---|
| Rust error | `rust/crates/sarmg-error` |
| SQLite baseline | `rust/crates/sarmg-sqlite` |
| JSON guard/schema | `packages/contracts` |
| timeout/body/error | `packages/http-client` |
| CSS/TS token | `packages/design-tokens` |
| 发布/依赖 | root manifests、scripts、workflows |

## 10.3 练习

1. 构造 ErrorCode 正负边界。
2. 在临时 SQLite 检查 pragma 和 busy。
3. 用同一 fixture 对 guard 和 JSON Schema 验证。
4. 模拟 chunked 超限 HTTP 响应。
5. 从 package tarball 而非 workspace import。
6. 断网运行一个构建后的消费者。

## 10.4 术语

| 术语 | 含义 |
|---|---|
| build-time dependency | 编译/打包时取得，生产无需联网 |
| wire contract | 跨进程或语言传输的数据语义 |
| runtime guard | 对运行时 unknown 值的真实检查 |
| exact keys | 只允许明确定义字段 |
| re-export | 从另一 package 再导出依赖符号 |
| baseline | 不声称拥有产品生命周期的安全基础配置 |
| WAL | SQLite Write-Ahead Log |
| package export | 消费者被允许使用的正式入口 |
| immutable release | 发布后不覆盖的版本制品 |
| consumer | 编译/打包共享库的真实产品 |
| compatibility alias | 为另一 API 名称保留的额外入口，本仓不提供 |

## 10.5 学成标准

能拒绝不合适的共享提案；能解释 type/guard/schema 差别；能说明 SQLite helper 不保证什么；能从发布包
验证 exports；能设计一次无 alias 的破坏性变更并列出所有消费者门禁。

## 10.6 后续阅读

共享能力决策见[工作流程](../project-workflow.md)，精确组件范围见[功能与取舍](../feature-inventory-and-tradeoffs.md)，
工具链、发布与安全事件见[运维文档](../operations.md)。
