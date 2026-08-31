# Sarmg Foundation

Sarmg Foundation `0.3.1` 是 Sarmg 产品的构建期共享层。它把已经由多个产品证明一致的安全原语、跨语言
wire contract、SQLite Schema 身份算法、管理员 Web 基线、设计令牌和发布校验工具集中维护。消费者在
编译或打包时锁定一个不可变版本并把代码带进自己的制品；生产环境不连接 Foundation，也不依赖本仓库、
GitHub、npm registry 或任何中央认证服务在线可用。

本版本只定义一套当前合同：不提供旧名称、旧字段、旧 Schema、deprecated wrapper、双读写、隐式降级或
兼容 fallback。历史状态的识别、备份、恢复和升级属于独立的 `sarmg-upgrade` 仓库；各在线产品只读取其
当前状态。Foundation 本身无 daemon、无业务数据库、无用户表、无 Session 表、无文件树，也不拥有产品的
发布周期。

## 1. 当前组件

### 1.1 Rust crate

| 组件 | 当前职责 | 明确不负责 |
|---|---|---|
| `sarmg-admin-auth` | 管理员 username、密码/Argon2id、Session/CSRF token、Cookie 提取、Origin/Host/Sec-Fetch-Site 同源校验 | 用户表、Session 持久化、Cookie 名称/TTL/属性、登录限流、审计、框架 middleware |
| `sarmg-contracts` | 管理员登录/Session、State、Release、Backup、Error 的严格 Rust wire 类型与共享 fixture | 产品业务 DTO、HTTP router、历史 manifest reader、物理路径检查 |
| `sarmg-error` | 有界 `ErrorCode`、`RequestId`、严格 `ErrorEnvelope`、常用 HTTP status/retry 默认值 | 产品错误码全集、日志脱敏、Axum rejection 和响应 middleware |
| `sarmg-schema-identity` | 驱动无关的五列 `product_metadata`、Schema fingerprint v1、精确 current identity 校验 | 打开数据库、执行 DDL/migration、路径安全、业务 Schema |
| `sarmg-server-target` | 在编译期把所有 Sarmg Server 限定为 `x86_64-unknown-linux-gnu`，并导出唯一 target 常量 | 限制 Android/iOS/Windows/macOS/Linux Agent 等客户端；构建或安装 Server |
| `sarmg-sqlite` | SQLx existing/create 显式打开、固定 PRAGMA、integrity/FK/checkpoint、Schema identity adapter | 产品实例锁、业务 transaction、初始化 DDL、backup/restore、文件 no-follow |

### 1.2 npm package

| 组件 | 当前职责 | 明确不负责 |
|---|---|---|
| `@sarmg/admin-web` | 管理员 API client、内存 Session、认证竞态控制、React hook、React/Vite/Node 精确工具链基线 | 登录页面、路由、品牌、Cookie 服务端实现、密码散列、持久化浏览器 token |
| `@sarmg/contracts` | TypeScript 类型、strict runtime guard、5 份 JSON Schema、跨 Rust/TS fixture | 宽松解析、历史合同、产品业务 response schema |
| `@sarmg/http-client` | same-origin JSON、cookie credential、CSRF、timeout/abort、响应字节预算、严格错误与 Retry-After | Session store、自动 mutation retry、跨 origin、文件上传下载、业务响应 guard |
| `@sarmg/design-tokens` | light/dark 语义 token、scoped reset、键盘/动态效果/forced-colors 可访问性基线 | UI 组件库、产品品牌、主题状态、字体、页面布局、全局 reset |

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
- 所有业务 Server 只允许 `x86_64-unknown-linux-gnu`。此限制不应用于客户端、Agent、移动端库，也不意味着
  Foundation 自己是一个 Server。
- Dufs 以外的产品 Web 统一 React `19.2.8`、React DOM `19.2.8`、Vite `7.3.6`、React plugin `4.7.0`、
  TypeScript `5.8.3`、Node `26.7.0`；Dufs 因单 binary 嵌入和文件管理前端边界保留原生 ES modules。
- current-only 是接受规则，不是自动迁移规则。遇到非当前 Schema、密码散列、字段或发布树时必须停止，
  不得在在线产品中添加特殊处理。

## 3. 仓库目录

```text
sarmg-foundation/
├─ rust/crates/
│  ├─ sarmg-admin-auth/
│  ├─ sarmg-contracts/
│  ├─ sarmg-error/
│  ├─ sarmg-schema-identity/
│  ├─ sarmg-server-target/
│  └─ sarmg-sqlite/
├─ packages/
│  ├─ admin-web/
│  ├─ contracts/
│  ├─ design-tokens/
│  └─ http-client/
├─ consumers/                 # 真实消费者采用状态与机器可验证 Schema
├─ scripts/                   # 稳定命令入口
├─ tools/                     # policy、package、release-tree 的严格实现与测试
└─ docs/                      # 中文学习、流程、功能边界和运维文档
```

`packages/` 是发布依赖，不是可运行客户端；因此不放入其他产品统一使用的 `clients/web`。本仓没有运行时
`config/`、`deploy/` 或 `clients/`，因为它没有需要部署的 Server、配置文件或产品 UI。

## 4. 固定工具链

| 工具 | 当前精确值 | 事实源 |
|---|---:|---|
| Rust | `1.98.0`，edition 2024 | `rust-toolchain.toml`、`Cargo.toml` |
| Node | `26.7.0` | `.node-version`、`engines.node`、CI |
| pnpm | `10.12.1` | 根 `packageManager`、CI |
| TypeScript | `5.8.3` | package manifest、lockfile |
| Foundation 版本 | `0.3.1` | Cargo/npm/package/release policy |

这些值是发布输入，不是“最低能运行即可”的建议范围。升级任一工具链都要同步 policy、lock、CI、package
smoke 和所有消费者验证。

## 5. 统一验证入口

用户要求先完成全部代码，再统一运行。代码冻结后从仓库根执行：

```bash
python3 scripts/check-foundation.py
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

`package-artifacts.py smoke` 会安全清理旧 `dist`、构建 4 个 package、检查所有 export、生成真实 `.tgz`、
审计 tar member，再在临时空目录离线安装并解析每个公开入口。workspace 中能 import 但 tarball 不能安装，
不算通过。

`check-rust-package-licenses.py` 会调用 Cargo 查看六个真实 crate 的 package 清单，并要求每个包根恰好包含
一个 `LICENSE`。repository policy 同时要求这六个文件都是普通、单链接文件，且字节与经过摘要固定的根
Apache-2.0 文本完全一致；因此 Git dependency 经 `cargo vendor` 展平后仍保留可审计许可证，不依赖消费者
仓库的通用 license fallback。

## 6. 发布与消费

1. Rust 消费者在联调阶段可暂用本地 `path`；正式提交必须使用 Foundation tag 对应的完整 40 位 commit，
   并同时声明 `version = "=0.3.1"`。
2. Web 消费者在联调阶段可暂用 `file:`；正式提交必须改成 GitHub Release 中经过校验的 `.tgz` URL并重建
   `package-lock.json`。消费者继续使用 npm，不因 Foundation 内部使用 pnpm 而改变。
3. `@sarmg/admin-web` 的产品通常还要显式锁定 `contracts`、`http-client`、`design-tokens` 和其 React/Vite
   peers；不能依赖 sibling workspace 偶然解析。
4. 普通 CI 只有 `contents: read`。只有精确 `v0.3.1` tag 的专用 release job 可获得 `contents: write`。
5. 发布资产包含 4 个 npm tarball、确定性 release-tool tarball、state contract、release identity、build
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
