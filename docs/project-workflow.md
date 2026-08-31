# Sarmg Foundation 工作流程与流程树

## 1. 总流程树

```text
共享需求
├─ 只有一个消费者 -> 留在产品本地
└─ 至少两个真实消费者
   ├─ 对齐语义/安全/故障边界
   ├─ 设计最小当前 API
   ├─ Rust/TypeScript + tests + distributable output
   ├─ Foundation CI
   ├─ 精确版本发布
   └─ 每个产品独立升级、集成测试并编入自身制品
```

## 2. 需求进入流程

提案必须列出真实消费者和重复实现、哪些部分完全相同、哪些必须留在产品，以及失败语义。评审重点
不是“能否抽象”，而是“抽象后是否仍保留最强消费者的安全约束”。没有第二消费者或只有未来猜测时，
拒绝共享。

## 3. Rust crate 流程

```text
API/type change
 -> unit tests: 正例 + 边界 + 负例
 -> cargo fmt/check/clippy/test workspace
 -> consumer 用精确 path/released version 集成
 -> consumer 自己验证 Schema/HTTP/运行时语义
```

`sarmg-sqlite` 只配置连接 baseline，消费者在外层拥有路径锚定、instance/maintenance lock、当前 Schema
fingerprint、业务 transaction 和备份。`sarmg-error` 只定义 error wire primitive，产品拥有 error code
namespace、日志和 HTTP middleware。

## 4. Web package 流程

```text
src + schema/CSS
 -> TypeScript typecheck
 -> build dist
 -> copy JSON Schema/CSS static exports
 -> tests import dist/package exports
 -> pnpm workspace verifies dependency graph
```

`@sarmg/http-client` 从 `@sarmg/contracts` 精确 workspace version 导入 `ErrorEnvelope`，不 re-export；
调用者也要显式依赖 contracts。这样所有权清晰，移除或升级某一包不会靠隐藏 alias 维持旧代码。

## 5. Breaking change 流程

0.x 变更直接定义新的当前 API：删除旧 type/function/export，更新 package version、锁文件、测试和全部
消费者。仓库不保留 deprecated wrapper、旧名称 alias 或 dual schema。若历史数据需要转换，那是
`sarmg-upgrade` 或消费产品的离线问题，不属于 Foundation runtime。

## 6. 消费者采用流程

1. 锁定精确 Foundation 版本，不依赖 branch、floating tag 或 CDN。
2. 在产品适配层显式导入，不让 Foundation 越权访问产品状态。
3. 运行产品自己的协议、数据库、UI 与 release 测试。
4. 构建后确认制品不需要 Foundation 仓库/registry/服务。
5. 若共享实现不足，先留在产品本地；不要降低产品约束来迁就共享 API。

## 7. CI 流程

Rust job 固定工具链，执行供应链门禁、format/check/clippy/test。Web job固定 Node/pnpm，使用 frozen
lockfile，执行 typecheck/build/test。workflow action 必须锁定 commit、最小只读权限、有 timeout 且
checkout 不保留凭据。

## 8. 删除流程

没有真实消费者、弱于产品实现或职责错误的组件直接从当前版本删除；同步删 manifest member、package
export、lockfile 与文档。消费者必须显式更新，不建立兼容墓地。删除共享库不会删除已编译产品中的代码。
