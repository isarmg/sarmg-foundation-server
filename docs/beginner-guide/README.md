# Sarmg Foundation 初学者学习指南

本手册用十章说明共享代码为何存在、如何发布、消费者承担什么责任，以及怎样避免把 Foundation 变成
新的运行时单点或兼容层。下方单页内容作为速览，专题章节是设计和评审依据。

1. [项目定位、边界与目录](01-project-overview.md)
2. [工具链、安装与第一次验证](02-environment-and-first-validation.md)
3. [Rust 错误与 SQLite 基础](03-rust-error-and-sqlite-basics.md)
4. [Contracts 类型、Guard 与 JSON Schema](04-contracts-types-guards-and-json-schema.md)
5. [HTTP Client 请求生命周期](05-http-client-request-lifecycle.md)
6. [Design Tokens 与消费者集成](06-design-tokens-and-consumer-integration.md)
7. [版本、发布与破坏性变更](07-versioning-publishing-and-breaking-changes.md)
8. [测试、调试与新增共享能力](08-testing-debugging-and-contribution.md)
9. [供应链、安全与维护运维](09-supply-chain-security-and-operations.md)
10. [源码路线、练习与术语表](10-reading-roadmap-and-glossary.md)

以下保留单页速览。

## 1. 什么是 Foundation

Foundation 是源码/包级复用仓库，不是“所有产品必须在线连接的基础服务”。消费者在构建时取得一个
精确版本，将代码编译/打包到自己的制品；上线后即使 Foundation 仓库、registry 或网络不可用，产品
仍必须完整运行。

只有至少两个真实产品拥有相同需求、边界和测试时，能力才适合共享。身份、业务 Schema、路由、运行
锁和迁移通常高度产品化，不应为了减少几行代码强行抽象。

## 2. Monorepo 结构

Rust 使用 Cargo workspace，Web 使用 pnpm workspace：

```text
sarmg-foundation
├─ rust/crates/
│  ├─ sarmg-error
│  └─ sarmg-sqlite
└─ packages/
   ├─ contracts
   ├─ http-client
   └─ design-tokens
```

Rust workspace edition 2024；Web 是 ESM package，TypeScript 编译到各自 `dist/`。

## 3. Rust：错误合同

`sarmg-error` 提供：

- `ErrorCode`：最长 128 bytes，以小写 ASCII 字母开头，只含小写字母、数字、`.`、`_`、`-`。
- `ErrorEnvelope`：`code`、展示用 `message`、可选 `request_id`、`retryable` 和对象型 `details`。
- `HttpStatus`：明确的 400/401/403/404/409/422/429/500/503 映射。

客户端必须按 `code`/HTTP status 分支，不能解析展示文字。Secret 和内部诊断只进服务日志，不能放进
envelope。

## 4. Rust：SQLite baseline

`sarmg-sqlite::open_pool` 配置 SQLx：WAL、foreign keys、5 秒 busy timeout、`synchronous=FULL`、10 秒
acquire timeout 与调用方给出的正连接数。另提供 integrity、foreign key check 与会报告 busy 的
TRUNCATE checkpoint。

它会 `create_if_missing`，因此消费者必须在调用前完成自己的路径、安全、锁、当前 Schema 与“是否
允许创建”判断。Foundation 不知道任何产品表，也不执行 migration/backup/restore。

## 5. Web：contracts

`@sarmg/contracts` 提供 TypeScript 类型、运行时 guard 和四个 JSON Schema：error envelope、state
contract、release、backup manifest。`isStateContract` 要求 exact keys、40 位小写 revision、SHA-256、
资源/外部 Secret/companion 列表结构，拒绝宽松对象。

Wire contract 的 TS 类型不是运行时验证。外部 JSON 必须先经过 guard 或 JSON Schema validator。

## 6. Web：http-client

`requestJson<T>` 默认同源 credential、10 秒 timeout、2 MiB 成功响应上限；允许上限最多 64 MiB。
它验证 JSON Content-Type，以 streaming reader 执行实际字节上限，规范解析 `ErrorEnvelope` 与
`Retry-After`，只给 unsafe method 添加有效 CSRF，并把 401 回调与权威 API error 隔离。

```ts
import { requestJson, ApiClientError } from "@sarmg/http-client";

const host = await requestJson<Host>("/api/v2/hosts/1", {
  csrfToken: session.csrfToken,
  onUnauthorized: () => session.clear(),
});
```

泛型 `T` 不会自动验证成功响应业务结构；消费者仍需自己的 schema/guard。

## 7. Web：design-tokens

包导出 TypeScript `tokens` 和 `tokens.css`/`tokens.dark.css`。当前仅含经过实际使用的颜色、间距和圆角。
它不提供组件、全局 reset、字体加载、主题状态管理或远程 CDN。

## 8. 开发工作流

```bash
cargo +1.98.0 test --workspace
pnpm install --frozen-lockfile
pnpm typecheck && pnpm build && pnpm test
```

Web build 必须产生可发布 `dist`；测试从已构建输出导入，防止源码能工作而 package export 失效。

## 9. 引入共享能力的判断

先在产品中证明需求与安全边界，再确认第二个真实消费者，并写出稳定最小 API、负例、版本破坏策略和
退出方案。若共享实现比产品本地实现弱、需要运行时依赖、或要携带旧 API alias，则不应进入 Foundation。

## 10. 术语

- **build-time dependency**：只在编译/打包阶段需要，上线不联网加载。
- **wire contract**：跨进程/语言传输的数据形状与语义。
- **runtime guard**：对 `unknown` 数据执行的真实运行时检查。
- **re-export**：一个包再次导出另一个包的符号；会隐藏依赖所有权并扩大兼容面。
- **WAL**：SQLite Write-Ahead Log。
- **0.x**：语义化版本中尚未承诺稳定公共 API 的阶段。
