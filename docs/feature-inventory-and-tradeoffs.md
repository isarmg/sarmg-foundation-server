# Sarmg Foundation 完整功能与取舍清单

## 0. 开发者决策台账

Foundation 是源代码依赖库，不拥有业务运行时。分类取“核心、保障、可选、建议保留、开发运维”；复杂度包含全部消费项目的迁移和验证成本。删除公开导出必须按破坏性新版本处理，不提供旧包名、旧类型或旧 CSS 变量 alias。

| ID | 功能/特性与当前实现 | 实现/主要依赖 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证 |
|---|---|---|---|---|---|---|
| FND-001 | Rust `sarmg-error` 稳定错误信封 | `rust/crates/sarmg-error`、serde | 核心 | 中 | Rust 服务需各自定义 wire error，跨项目语义漂移 | 序列化、拒绝未知字段、边界 |
| FND-002 | 错误 code/message/details 分层 | error types、JSON contract | 保障 | 中 | UI 只能解析自由文本或可能泄露内部错误 | public/internal 映射测试 |
| FND-003 | Rust `sarmg-sqlite` Schema 指纹 | sqlite crate、rusqlite | 核心 | 高 | 产品无法使用统一规范 DDL identity | 顺序、internal objects、漂移 fixture |
| FND-004 | SQLite integrity/FK/metadata 验证原语 | sqlite crate | 保障 | 高 | 各产品可能漏检损坏或错版本库 | corrupt/FK/metadata 负例 |
| FND-005 | 安全路径/普通文件辅助边界 | sqlite/file helpers | 保障 | 高 | 调用方需重复实现 symlink/hardlink/权限检查 | race、mode、link 测试 |
| FND-006 | `@sarmg/contracts` Error schema | `packages/contracts`、JSON Schema | 核心 | 中 | Web/工具无法共享错误格式 | TS guard + schema validator |
| FND-007 | State contract schema | contracts schemas/types | 核心 | 中 | 状态机/operation 投影需各自命名 | 合法/非法状态 fixture |
| FND-008 | Release identity schema | contracts schemas/types | 保障 | 中 | 发布工具无法统一校验产品/版本/SHA | canonical hash、unknown field |
| FND-009 | Backup manifest schema | contracts schemas/types | 保障 | 高 | 备份工具和产品无法共享资源描述边界 | path/hash/count/requirement 负例 |
| FND-010 | Runtime TypeScript guards | contracts `src/index.ts` | 保障 | 中 | TS 静态类型无法保护不可信 JSON | unknown 输入、嵌套、额外字段 |
| FND-011 | 构建时复制 Schema 到 dist | copy-schemas、package files | 开发运维 | 低 | 发布包缺少 JSON Schema，消费者只能用 TS | clean pack 内容测试 |
| FND-012 | `@sarmg/http-client` same-origin JSON client | fetch wrapper、contracts | 核心 | 中 | 每个 Web 重写 timeout/body/error 解析 | 2xx/error/abort/invalid JSON |
| FND-013 | HTTP timeout/AbortSignal 合并 | http-client | 保障 | 高 | 慢请求无限挂起或调用方取消失效 | caller abort、timeout、竞态 |
| FND-014 | 有界响应正文读取 | http-client | 保障 | 中 | 错误或 JSON 响应可耗尽浏览器内存 | Content-Length/chunked 超限 |
| FND-015 | 严格 Content-Type/JSON/error envelope | http-client、contracts | 保障 | 中 | HTML 代理错误可能被当业务 JSON | MIME、空体、畸形 JSON |
| FND-016 | same-origin URL 限制 | http-client URL resolver | 保障 | 中 | credentialed request 可发往第三方 origin | scheme/origin/userinfo 测试 |
| FND-017 | `@sarmg/design-tokens` TypeScript tokens | design-tokens package | 建议保留 | 低 | 各 UI 颜色/间距命名重新分叉 | export 与消费 build |
| FND-018 | 明/暗色 CSS 变量 | tokens.css、tokens.dark.css | 建议保留 | 中 | Web 主题一致性和可访问性降低 | light/dark 变量完整性 |
| FND-019 | reset/accessibility 设计基线 | CSS bundle/消费者 vendoring | 建议保留 | 中 | 各项目重新处理 focus、字体和控件差异 | keyboard/focus/contrast |
| FND-020 | 只发布构建输出和必要 Schema/CSS | package `files`、exports | 保障 | 低 | 源码/测试意外进入包或运行期文件缺失 | `pnpm pack --dry-run` |
| FND-021 | 精确 workspace 内部依赖 | pnpm workspace、Cargo workspace | 保障 | 中 | 本地与发布解析可能选择不同版本 | frozen lock、pack/install fixture |
| FND-022 | Rust/TS 独立包版本一致性门禁 | root scripts、package metadata | 开发运维 | 中 | 同一 Foundation release 内部版本漂移 | version verification script |
| FND-023 | Rust fmt/clippy/test 门禁 | Cargo workspace、CI | 开发运维 | 中 | 安全原语回归进入消费者 | all-targets clippy/test |
| FND-024 | TS type/build/test 门禁 | pnpm scripts、CI | 开发运维 | 中 | Schema、guards 和 exports 漂移 | frozen install、typecheck、tests |
| FND-025 | 中文学习、流程、功能和运维文档 | README、`docs/` | 开发运维 | 低 | 消费者误用底层原语或复制错误模式 | 链接/API 示例抽查 |
| FND-026 | 明确不拥有认证、业务 DB、UI 应用、部署和历史兼容 | 仓库边界 | 核心 | 高 | 引入这些能力会把基础库变成中央运行时并扩大耦合 | 新能力必须单独架构评审 |

## 1. 组件清单

| 组件 | 提供 | 不提供 |
|---|---|---|
| `sarmg-error` | bounded ErrorCode、ErrorEnvelope、HTTP status/retry default | Axum middleware、日志、产品错误码、兼容 alias |
| `sarmg-sqlite` | WAL pool、FK/FULL/busy/acquire baseline、integrity/FK/checkpoint | Schema、migration、路径安全、锁、backup/restore |
| `@sarmg/contracts` | TS 类型、runtime guard、四类 JSON Schema | 产品业务 DTO、宽松 legacy schema、HTTP client re-export |
| `@sarmg/http-client` | bounded JSON、timeout/abort、CSRF、typed error、Retry-After | 业务 response validator、auth store、retry loop、非 JSON 下载 |
| `@sarmg/design-tokens` | 小型颜色/间距/圆角 token、light/dark CSS | 组件库、全局 shell、字体、主题状态、CDN runtime |

## 2. 已删除且不兼容保留的方向

认证、HTTP middleware、配置、observability、路径验证、operation types，以及 Web shell/auth/testkit/
global API prefix 等曾评估的共享方向没有足够真实消费者或弱于产品本地边界，因此不在当前仓库。旧
`ApiError` alias、`http-client` 的 `ErrorEnvelope` re-export 等 0.1 入口也不保留。

## 3. 关键取舍

- 小而明确优于“大一统平台”：重复少量 glue code，换取产品安全边界独立。
- build-time 优于 runtime central service：减少在线故障域，代价是升级要逐产品重建。
- exact runtime guard 优于仅 TypeScript 类型：能处理外部不可信 JSON，代价是维护类型与 guard/Schema。
- 有界 HTTP client 优于原生 `response.json()`：防止无界响应，代价是 buffered JSON 仍受最大 64 MiB 限制。
- SQLite baseline 不拥有 lifecycle：避免一个通用库误判产品 Schema，代价是消费者必须实现强外层。

## 4. 稳定性与版本

全部组件为 `0.2.0` 实验版本。任何 breaking change 都可在新当前版本直接删除旧 API；不会添加兼容
wrapper、deprecated alias 或 dual contract。只有至少两个消费者稳定采用、API/安全边界成熟后，才
评估 `1.0`。

## 5. 安全边界

Foundation 不能让弱默认覆盖产品强约束。Error message/details 不放 Secret；HTTP client 不把任意非
JSON error body暴露为异常；SQLite helper 不声称路径或 Schema 安全；JSON guard 拒绝未知结构。依赖
升级需运行所有消费者集成测试，而不仅是本仓库 unit test。

## 6. `sarmg-error` 详细合同

| 能力 | 当前行为 | 消费者仍需负责 |
|---|---|---|
| `ErrorCode` | 小写 ASCII、首字符/字符集/128 bytes 上限 | 设计产品 namespace 与每个业务 code |
| `ErrorEnvelope` | code/message/request_id/retryable/object details | Secret redaction、国际化和日志关联 |
| HTTP status | 明确常见 4xx/5xx primitive | route/middleware 和业务映射 |
| Serialization | 跨进程稳定的最小结构 | Content-Type、body size、response headers |
| Retry hint | 基础 retryable 语义 | 幂等、Retry-After、operation unknown 策略 |

选择小 primitive 而非 Axum middleware，避免把不同产品的认证、request ID、日志和 body policy 强行统一。

## 7. `sarmg-sqlite` 详细合同

| 提供的 baseline | 精确作用 | 不构成的保证 |
|---|---|---|
| WAL | 改善读写并行 | 不等于只复制 main file 可备份 |
| foreign keys ON | 连接级启用约束 | 不证明现有数据/Schema 正确 |
| synchronous FULL | 更强提交持久性 | 不替代目录 fsync/组合状态 journal |
| busy timeout 5s | 有界等待锁 | 不解决多 writer 架构 |
| acquire timeout 10s | 有界连接池等待 | 不替代业务 backpressure |
| integrity/FK check | 诊断数据库结构/引用 | 不自动修复 |
| TRUNCATE checkpoint | 安全窗口尝试收敛 WAL | busy 不能当成功 |

消费者调用前必须拥有路径锚定、普通文件/权限、实例锁、maintenance 锁、当前 metadata/DDL、创建策略；
调用后仍拥有 transaction、writer、backup/restore 和业务 doctor。

## 8. `@sarmg/contracts` 详细合同

| Export | 用途 | 验证层次 | 限制 |
|---|---|---|---|
| Error envelope type/guard/schema | 统一最小 API 错误形状 | TS + runtime + JSON Schema | 不分配产品错误码 |
| State contract | 描述一个产品状态代 | exact keys、revision/SHA/resource | 不执行状态发现或转换 |
| Release schema | 描述不可变制品身份 | JSON 结构 | 不验证物理路径/Hash 本身 |
| Backup manifest schema | 描述备份资源和 external 要求 | JSON 结构 | 不认证 SQLite/密文/tree |

类型、guard 和 Schema 必须同步。外部 `unknown` 先用 guard/schema，不能因为代码写了泛型就跳过验证。

## 9. `@sarmg/http-client` 详细合同

| 阶段 | 当前能力 | 取舍 |
|---|---|---|
| 请求 | same-origin credential、unsafe CSRF | 不拥有 auth store/base URL |
| 取消 | caller abort + 10 秒默认 timeout | 不断言 mutation 未执行 |
| 成功正文 | JSON Content-Type、streaming 2 MiB 默认 | 最大 64 MiB，仍是内存 JSON |
| 错误正文 | 严格 envelope、安全 fallback | 不暴露任意 HTML/text 上游正文 |
| 401 | 可调用消费者 callback | callback 不覆盖权威错误 |
| Retry-After | 规范解析 | 不自动重试 |
| 泛型 | 编译期调用体验 | 不提供运行时业务 response guard |

通用 retry 被明确排除，因为 GET、幂等 operation、普通 POST 和 unknown 副作用的安全策略不相同。

## 10. `@sarmg/design-tokens` 详细合同

| 类别 | 提供 | 不提供的原因 |
|---|---|---|
| 颜色 | 被真实消费者复用的语义 token | 不接管产品品牌与组件状态 |
| 间距/圆角 | 小型稳定 primitive | 不形成完整 layout system |
| Light/Dark CSS | `--sarmg-*` custom properties | 主题选择属于消费者状态 |
| TypeScript tokens | 图表/程序化样式值 | 不加载运行时配置/CDN |
| Package exports | CSS/JS/types 正式入口 | 不允许深层源码 import |

共享包不包含全局 reset、字体、组件、图标、路由或 localStorage。新增 token 要有两个真实消费者、light/
dark 语义和可访问性验证。

## 11. 消费者采用矩阵

| 阶段 | Foundation 证据 | 消费者证据 |
|---|---|---|
| 选型 | 最小 API、边界和负例 | 两个真实场景确实相同 |
| 锁定 | immutable crate/package version | manifest/lock 精确依赖 |
| 构建 | workspace gates、dist/exports | 产品 compile/bundle |
| 集成 | 通用 fixture | 产品协议/Schema/UI/安全测试 |
| 发布 | package inventory/checksum | 产品自身不可变发行物 |
| 运行 | 无在线服务 | 断网仍完整运行 |
| 升级 | breaking change 清单 | 全部调用点更新且无 alias |

## 12. 候选共享能力决策

| 候选 | 当前结论 | 关键理由 |
|---|---|---|
| 认证/Session | 留在产品 | Cookie、角色、TTL、代理和 threat model 不同 |
| HTTP middleware | 留在产品 | request ID、body、身份、错误映射不同 |
| 产品配置框架 | 留在产品 | 环境、Secret、路径、生产边界不同 |
| Observability runtime | 留在产品 | 指标/日志字段和 exporter 生命周期不同 |
| 路径安全 | 留在产品 | Linux `openat2`、平台和资源语义不同 |
| Operation types | 留在产品 | 外部副作用和状态机不同 |
| UI shell/auth store | 留在产品 | 页面/会话/构建体系不一致 |
| Error primitive | 共享 | 语义小且多个产品一致 |
| SQLite connection baseline | 共享 | pragma/诊断基础一致且不越权 |
| State/release schemas | 共享 | 跨工具传输结构一致 |

## 13. 破坏性变更的影响面

| 变化 | 必须同步 |
|---|---|
| crate/package/仓库名称 | workspace、lock、import、scope、CSS、Schema example、CI、消费者 |
| ErrorEnvelope | Rust/TS/Schema、API fixture、全部客户端 |
| State/Release/Backup Schema | guard、JSON Schema、升级工具、产品发行验证 |
| HTTP timeout/size/error | 所有 Web 消费者的失败和内存测试 |
| SQLite pragma/API | 全部产品数据库并发、doctor 和恢复测试 |
| Token | CSS/TS、light/dark、产品视觉/可访问性测试 |

0.x 新版直接删除旧 export/字段/property，不保留 deprecated wrapper、dual schema 或 alias package。

## 14. 质量与发布完成定义

组件只有在公开 API 最小、正负测试完整、构建输出可从 package export 使用、包内容受控、依赖锁固定、
供应链门禁通过、至少两个消费者完成真实集成并断网运行、中文文档说明边界后才算可发布。workspace 内部
编译成功但 tarball/export 或消费者失败，不算完成。
