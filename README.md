# Sarmg Foundation Server

本仓只规定 Server、管理 Server 的 Web 和服务端离线维护工具的行为。Client、Android/iOS、管理客户端自身的 Web 及移动 FFI
由独立的 `sarmg-foundation-client` 仓库规定；两个基础仓库互不依赖，不保留旧仓库名入口。

Sarmg Foundation Server 是 Sarmg 服务端及管理 Web 的上游平台规范、基础实现、工具链和一致性验证系统。产品实现可以
作为 Foundation 的设计输入；一项能力进入 Foundation 后，Foundation 就成为唯一事实源，产品必须通过
Profile、Capability 和业务 Adapter 使用它，不得要求 Foundation 永久兼容产品本地实现，也不得继续维护
第二套平台能力。

Foundation 是构建期中央平台，不是生产环境中的中央服务。每个产品仍编译、发布并独立运行自己的正式
二进制；消费者锁定 Foundation 的精确版本和不可变 Git revision，并将需要的实现带入自身制品。生产环境
不连接 Foundation，也不依赖本仓库、GitHub、包注册表或中央认证服务在线可用。

当前工作区以 `0.7.1` 为版本基线，仅提供服务端及其管理 Web 的平台实现；P11/P12 客户端实现归 Client 仓库。
产品 Server 与 Client 使用独立后缀仓库，原不可变标签保持不变。产品采用状态由消费者矩阵记录，
这不是 Foundation 1.0 完成声明。

本版本只定义一套当前合同：不提供旧名称、旧字段、旧 Schema、deprecated wrapper、双读写、隐式降级或
兼容 fallback。`sarmg-upgrade` 仅承担服务端当前状态的离线维护；不实现历史版本升级。各在线产品只读取当前状态。Foundation 本身无 daemon、无产品业务数据库、无产品业务表和文件树，也不拥有产品的发布周期；
但平台数据库 DDL、管理员与 Session 机制、Server/Web Runtime 等平台能力由 Foundation 定义和发布。
不要求任何仓库实现历史状态读取或旧版本转换。

## 1. 当前迁移基线组件

### 1.1 Rust crate

| 组件 | 当前职责 | 明确不负责 |
|---|---|---|
| `sarmg-admin-auth` | 迁移前管理员 username、密码/Argon2id、Session/CSRF token、Cookie 与同源检查 primitive | 目标能力由 Admin Core、Store 和 HTTP Adapter 取代；产品不得据此永久保留本地控制面 |
| `sarmg-contracts` | 管理员登录/Session、State、Release、Backup、Error 的严格 Rust wire 类型与共享 fixture | 产品业务 DTO、HTTP router、历史 manifest reader、物理路径检查 |
| `sarmg-error` | 有界 `ErrorCode`、`RequestId`、严格 `ErrorEnvelope`、常用 HTTP status/retry 默认值 | 产品错误码全集、日志脱敏、Axum rejection 和响应 middleware |
| `sarmg-schema-identity` | 驱动无关的五列 `product_metadata`、Schema fingerprint v1、精确 current identity 校验 | 打开数据库、执行 DDL/migration、路径安全、业务 Schema |
| `sarmg-server-target` | 在编译期把所有 Sarmg Server 限定为 `x86_64-unknown-linux-gnu`，并导出唯一 target 常量 | 限制 Android/iOS/Windows/macOS/Linux Client 等客户端；构建或安装 Server |
| `sarmg-sqlite` | SQLx existing/create 显式打开、固定 PRAGMA、integrity/FK/checkpoint、Schema identity adapter | 产品实例锁、业务 transaction、初始化 DDL、backup/restore、文件 no-follow |
| `sarmg-state-file` | 带摘要、权限和原子替换约束的小型平台状态文件 | 产品业务文件树、历史格式转换 |
| `sarmg-platform-db` | 平台 metadata DDL、保留表前缀和当前 generation 验证 | 产品业务 DDL、在线 migration |
| `sarmg-admin-core` | 管理员政策、登录准入、Session/CSRF 状态机与 Store 合同 | 具体数据库、HTTP 框架、产品授权 |
| `sarmg-admin-sqlite` | 持久管理员、Session 和安全审计的 SQLite Store | 业务账户、业务审计 |
| `sarmg-admin-static` | 静态 PHC 管理员与进程内 Session Store | Web 管理员增删、跨重启 Session |
| `sarmg-admin-axum` | Foundation Auth Router、Cookie、Origin/CSRF 与 Axum 请求认证 | 产品业务路由和业务权限 |
| `sarmg-admin-hyper` | 与 Admin Core 同语义的 Hyper 请求/响应适配边界 | 文件服务业务实现 |
| `sarmg-server-runtime` | 进程身份、Request ID、健康、信号和后台任务监督（无诊断 HTTP 接口） | 产品 AppState 与业务 Router |
| `sarmg-fs-safety` | typed 相对路径、原子发布、目录预算和 Linux openat2 根 | 产品路径命名与文件内容语义 |
| `sarmg-secret` | 默认脱敏并在 drop 清零的内存秘密类型 | 密钥持久化和产品密钥轮换 |
| `sarmg-secret-envelope` | 域与对象绑定的有界 AES-GCM envelope | 产品域、对象 ID 和业务字段 |
| `sarmg-secure-http` | 三种固定网络策略、DNS/地址、超时与响应预算 | 产品 API DTO 和重试语义 |
| `sarmg-secure-xml` | DTD/ENTITY 拒绝及深度、节点、文本、时间预算 | ONVIF 类型与业务解析 |
| `sarmg-operations` | Durable Operation 状态、转移、幂等冲突和平台 DDL | 远端执行器与业务 payload |
| `sarmg-testkit` | Adapter 与消费者共用的认证协议验收断言（仅 dev-dependency） | 生产运行逻辑和产品业务夹具 |

### 1.2 npm package

| 组件 | 当前职责 | 明确不负责 |
|---|---|---|
| `@sarmg/admin-web` | 管理员 API client、内存 Session、认证竞态控制与 React hook | 构建工具链、登录页面、Cookie 服务端实现 |
| `@sarmg/contracts` | TypeScript 类型、strict runtime guard、5 份 JSON Schema、跨 Rust/TS fixture | 宽松解析、历史合同、产品业务 response schema |
| `@sarmg/http-client` | same-origin JSON、cookie credential、CSRF、timeout/abort、响应字节预算、严格错误与 Retry-After | Session store、自动 mutation retry、跨 origin、文件上传下载、业务响应 guard |
| `@sarmg/design-tokens` | light/dark 语义 token、scoped reset、键盘/动态效果/forced-colors 可访问性基线 | UI 组件库、产品品牌、主题状态、字体、页面布局、全局 reset |
| `@sarmg/web-toolchain` | 精确 Node/React/Vite/TS、tsconfig、Vite 输出和 source-map/体积策略 | 产品路由与 UI |
| `@sarmg/admin-ui` | 管理面基础控件、状态组件和安全交互 | 产品业务组件 |
| `@sarmg/admin-shell` | 登录/恢复、顶栏、导航、错误与 Toast 外壳 | 产品页面和业务路由 |
| `@sarmg/web-fonts` | 固定 Maple Mono commit、WOFF2、OFL、SHA-256 与 CSS 映射 | 设计 token 与产品品牌 |

### 1.3 管理 Web 默认外观

当前源码的 `@sarmg/admin-ui/styles.css` 默认提供 Union 内容块外观（3:2 六行卡片、
旧版色板及圆角），当前默认西文字体为 Maple Mono Normal NL 正体，中文/日文资源保持不变。
消费者可设置 `html[data-sarmg-appearance="custom"]` 或其他自定义名称并加载自己的 CSS。
默认外观不是强制品牌规范，自定义外观不能改变认证、会话、权限及无障碍要求。
详见 [默认外观与消费者自定义](packages/admin-ui/CONTENT-BLOCKS.md)。
当前正式 0.7.1 已包含这些展示层能力；四个控制平面消费者使用已验证的 0.7.0，Dufs 使用 0.7.1。消费者直接安装不可变 Release 包并锁定 integrity，不复制平台源码快照，不覆盖已发布制品。

## 2. 统一后的硬边界

- 管理面只有 `admin` 一种角色。数据库不需要角色列；wire 中固定 `role: "admin"`，不存在 viewer、operator
  或产品自定义管理角色。
- 管理员认证路径固定为 `/api/v2/auth/login`、`/api/v2/auth/session`、`/api/v2/auth/logout`。
- 管理员 username 登录候选限 1–64 个可打印 ASCII 字节，只做 ASCII trim/lower；这层只是有界 wire
  输入，不代表身份已被接受。持久值和 Session 必须为 3–64 字节、首尾字母或数字且只含
  `[a-z0-9._-]`。管理身份不是邮箱；`@`、Unicode、控制字符等候选会在 canonical admission 被拒绝。
- 密码散列只接受当前 Argon2id v19 参数：`m=19456,t=2,p=1`、16-byte salt、32-byte output；参数不同的
  合法 PHC 也会被拒绝。
- Session/CSRF token 是 32-byte 随机值的 URL-safe Base64 无填充编码，必须恰好 43 个字符；持久化摘要
  为 SHA-256，比较使用 constant time。
- 浏览器 mutation 必须提供唯一、规范且相互一致的 `Origin`、有效 `Host`/HTTP2 authority、
  `Sec-Fetch-Site: same-origin` 与当前 CSRF；生产只允许 HTTPS，本地 HTTP 只允许真实 loopback。
- 所有业务 Server 只允许 `x86_64-unknown-linux-gnu`。此限制不应用于客户端、Client、移动端库，也不意味着
  Foundation 自己是一个 Server。
- Dufs 以外的产品 Web 统一 React `19.2.8`、React DOM `19.2.8`、Vite `7.3.6`、React plugin `4.7.0`、
  TypeScript `5.8.3`、Node `26.7.0`；Dufs 因单 binary 嵌入和文件管理前端边界保留原生 ES modules。
- current-only 是接受规则，不是自动迁移规则。遇到非当前 Schema、密码散列、字段或发布树时必须停止，
  不得在在线产品中添加特殊处理。

## 3. 仓库目录

```text
sarmg-foundation-server/
├─ rust/crates/
│  ├─ sarmg-admin-auth/
│  ├─ sarmg-admin-axum/
│  ├─ sarmg-admin-core/
│  ├─ sarmg-admin-hyper/
│  ├─ sarmg-admin-sqlite/
│  ├─ sarmg-admin-static/
│  ├─ sarmg-contracts/
│  ├─ sarmg-error/
│  ├─ sarmg-platform-db/
│  ├─ sarmg-schema-identity/
│  ├─ sarmg-server-target/
│  ├─ sarmg-sqlite/
│  ├─ sarmg-state-file/
│  └─ server runtime/security/operations crates
├─ packages/
│  ├─ admin-web/
│  ├─ contracts/
│  ├─ design-tokens/
│  ├─ http-client/
│  └─ web-toolchain、admin-ui、admin-shell、web-fonts/
├─ profiles/                  # Foundation 发布的有限 Profile/Capability 组合
├─ schemas/                   # 产品清单及后续平台 DDL 的机器可读 Schema
├─ exceptions/                # 迁移期、到期且不降低安全下限的例外
├─ consumers/                 # 自动生成的真实消费者采用状态
├─ scripts/                   # 稳定命令入口
├─ tools/                     # policy、package、release-tree 的严格实现与测试
└─ docs/                      # 中文学习、流程、功能边界和运维文档
```

`packages/` 是发布依赖，不是可运行客户端；因此不放入其他产品统一使用的 `clients/web`。本仓没有运行时
`config/`、`deploy/` 或 `clients/`，因为它没有需要部署的 Server、配置文件或产品 UI。目录只在对应纵向
切片进入实现阶段时创建，不预建空 crate。

## 4. 固定工具链

| 工具 | 当前精确值 | 事实源 |
|---|---:|---|
| Rust | `1.98.0`，edition 2024 | `rust-toolchain.toml`、`Cargo.toml` |
| Node | `26.7.0` | `.node-version`、`engines.node`、CI |
| pnpm | `10.12.1` | 根 `packageManager`、CI |
| TypeScript | `5.8.3` | package manifest、lockfile |
| Foundation 版本 | `0.6.1` | Cargo/npm/package/release policy |

这些值是发布输入，不是“最低能运行即可”的建议范围。升级任一工具链都要同步 policy、lock、CI、package
smoke 和所有消费者验证。

## 5. 统一验证入口

用户要求先完成全部代码，再统一运行。代码冻结后从仓库根执行：

```bash
python3 scripts/check-foundation.py
python3 scripts/sarmg-conformance.py verify-foundation
python3 scripts/sarmg-conformance.py verify-consumers
python3 scripts/check-rust-package-licenses.py
python3 scripts/check-workflow-supply-chain.py
python3 -m unittest discover -s tools/tests -p 'test_*.py'
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --workspace --all-features --no-deps
pnpm install --frozen-lockfile --ignore-scripts
pnpm typecheck
pnpm test
python3 scripts/package-artifacts.py smoke
git diff --check
```

`package-artifacts.py smoke` 会安全清理旧 `dist`、构建 8 个 package、检查所有 export、生成真实 `.tgz`、
审计 tar member，再在临时空目录离线安装并解析每个公开入口。workspace 中能 import 但 tarball 不能安装，
不算通过。

`check-rust-package-licenses.py` 会调用 Cargo 查看二十二个真实 crate 的 package 清单，并要求每个包根恰好包含
一个 `LICENSE`。repository policy 同时要求这些文件都是普通、单链接文件，且字节与经过摘要固定的根
Apache-2.0 文本完全一致；因此 Git dependency 经 `cargo vendor` 展平后仍保留可审计许可证，不依赖消费者
仓库的通用 license fallback。

## 6. 发布与消费

1. Rust 消费者在联调阶段可暂用本地 `path`；正式提交必须使用 Foundation tag 对应的完整 40 位 commit，
   并同时声明 `version = "=0.6.1"`。
2. Web 消费者在联调阶段可暂用 `file:`；正式提交必须改成 GitHub Release 中经过校验的 `.tgz` URL并重建
   `package-lock.json`。消费者继续使用 npm，不因 Foundation 内部使用 pnpm 而改变。
3. `@sarmg/admin-web` 的产品通常还要显式锁定 `contracts`、`http-client`、`design-tokens` 和其 React/Vite
   peers；不能依赖 sibling workspace 偶然解析。
4. 普通 CI 只有 `contents: read`。只有精确 `v0.6.1` tag 的专用 release job 可获得 `contents: write`。
5. 发布资产包含 8 个 npm tarball、确定性 release-tool tarball、state contract、release identity、build
   inventory、`SHA256SUMS` 和 exact release-tree manifest。
6. Foundation verifier 只给最低共同边界。产品仍须验证自身目录 allowlist、mode、binary self-binding、
   资源总大小、配置与数据库身份。

## 7. 文档入口

- [文档总览](docs/README.md)
- [初学者学习指南](docs/beginner-guide/README.md)
- [项目工作流程与流程树](docs/project-workflow.md)
- [完整功能与取舍清单](docs/feature-inventory-and-tradeoffs.md)
- [仓库、依赖、发布与故障运维](docs/operations.md)

代码采用 [Apache License 2.0](LICENSE)。
