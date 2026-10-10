# xcss 运维文档

## 1. 运维范围

本仓库维护公共库、构建工具、不可变输入和共享检查。xcss没有生产daemon；这里只说明自己的来源、构建、发布、缓存和故障诊断。

## 2. 当前发行身份与事实源

1.0.0 发布账号设置内容块、可编辑日志日期范围和提前加载字体的启动逻辑。正式消费以发布后的完整源码 revision 与不可变资产为准；既有发行保持不变。xcss 物理合并为一个 Linux AMD64 Rust crate 与一个服务端管理 Web npm 包。详见[版本说明](releases/1.0.0.md)。

| 项目 | 唯一当前值 | 权威位置 | 漂移时的处理 |
|---|---|---|---|
| xcss 版本 | `1.0.0` | 根 `Cargo.toml`、`package.json`、根单包 manifests、policy | 阻止 CI/发布，统一更新后重建 lock |
| Rust | `1.99.0` | `rust-toolchain.toml` | 不用其他版本代替验证 |
| Rust edition/MSRV | 2024 / `1.99` | 根 Cargo package | 作为工具链大问题单独升级 |
| Node | `26.7.0` | `.node-version`、`engines.node`、CI | 切换 Node，不放宽 engine |
| pnpm | `10.34.6` | 根 `packageManager`、CI | 安装精确版本，不使用 Corepack 浮动解析 |
| TypeScript | `7.0.2` | 根 package.json、lock | 与产品 Web 基线一起升级 |
| React / React DOM | `19.3.0` | `admin-web` toolchain/peer/dev deps | 所有 React 管理 Web（包括 Xczs）同步验证 |
| Vite / React plugin | `8.3.3` / `6.1.2` | `admin-web` toolchain/peer/dev deps | 所有非 Xczs Web 同步验证 |
| Server target | `x86_64-unknown-linux-gnu` | `xcss::server_target` | Server 其他 target 编译必须失败 |
| License | Apache-2.0 | 根 `LICENSE`、Cargo/npm metadata、Cargo package 清单 | 缺失或字节漂移即不发布 |
| Release tag | `v1.0.0` | Git tag | 一经发布不移动、不覆盖、不重建同版本 |

xcss 是构建时服务端依赖库，不独立运行 daemon。整个 crate、Node 构建进程及 release identity 的目标均为
`x86_64-unknown-linux-gnu`；全部内部模块共享这个平台约束。

### 3.1 必需工具

```bash
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy
node --version
pnpm --version
python3 --version
git --version
```

Node 输出必须为 `v26.7.0`，pnpm 必须为 `10.34.6`。Python 与 Git 没有在本仓声明一个可发布 runtime，
但必须支持当前脚本和平台。首次安装依赖：

```bash
pnpm install --frozen-lockfile --ignore-scripts
cargo metadata --locked --no-deps --format-version 1
```

不要把 `cargo update`、`pnpm update`、删除 lockfile 或无 `--frozen-lockfile` 安装作为普通排障方式。依赖
更新需要独立评审，不应通过重新解析隐藏 manifest/lock 漂移。

### 3.2 可安全删除的缓存

在确认目标是本仓具体目录后，可以删除并重建 `target/`、根 `node_modules/`、根 `dist/` 和临时
release 输出。不可将 `Cargo.lock`、`pnpm-lock.yaml`、Schema、fixture、consumer matrix、文档或 Git tag
当缓存处理。不要对工作区根或未解析变量使用递归删除。

## 4. 统一质量门

代码与文档全部完成后，按顺序执行：

```bash
python3 scripts/check-xcss.py
python3 scripts/check-rust-package-licenses.py
python3 scripts/check-workflow-supply-chain.py
python3 -m unittest discover -s tools/tests -p 'test_*.py'
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --all-features --no-deps
pnpm install --frozen-lockfile --ignore-scripts
pnpm typecheck
pnpm test
python3 scripts/package-artifacts.py smoke
git diff --check
git status --short
```

失败后修复实际原因，从受影响层向下重跑；最终交付前再完整跑一次。禁止用以下方式“修复”失败：放宽
strict guard、接受另一套 Argon2 参数、忽略重复安全 header、提高到无界 body、删除竞态/攻击负例、添加
旧字段 alias、跳过 tarball isolated install + TypeScript/Vite 或把消费者矩阵手改为 conforming。

## 5. Repository Policy 运维

`scripts/check-xcss.py` 调用 `tools/xcss_policy.py`，当前核对：

- Cargo/npm/policy 的版本均为 `1.0.0`；
- Rust `1.99.0`、Node `26.7.0`、pnpm `10.34.6` 的事实源一致；
- 只有一个 Rust package `xcss` 和一个 npm package `@xcss/web`，没有子 package manifest 或内部 package 依赖；
- 根 `LICENSE` 必须是普通、单链接、审核过的 Apache-2.0 文本，Cargo 清单及 npm tgz实际携带它；
- 整个 crate 启用安全 lint 和 Linux AMD64 GNU 编译硬门禁；
- npm 的 `os`、`cpu`、`libc` 必须为 Linux、x64、glibc；CLI 和构建入口再次检查实际进程平台；
- `admin-web` 的 React/Vite/TypeScript/type package 精确一致；
- 源码和文档不存在已取消的项目/客户端名称。

消费者矩阵是独立接入报告，不参与 `check-xcss.py`、xcss CI 或 Release 门禁。维护报告时另外
运行 `python3 scripts/xcss-conformance.py verify-consumers` 与
`python3 scripts/xcss-conformance.py generate-consumer-matrix --check`；产品行为验收仍在对应产品仓库完成。

产品源码接入检查使用 `python3 scripts/xcss-conformance.py report --product-root <path> --json`。该报告会
逐项标明 manifest、依赖、Schema、Web 与 release 状态；没有 release 清单时状态是 `not-checked` 且
`release_verified=false`。正式产物门禁必须另行运行
`python3 scripts/xcss-conformance.py verify-release --product-root <path> --require-published`，并继续使用
`xcss-release verify` 核对实际发布树、文件模式、大小与 SHA-256。

产品 Rust xcss 依赖只接受官方 Git URL、精确版本和完整 revision，并由 `Cargo.lock` 复核实际解析
结果；Web xcss 依赖只接受对应版本的官方 release tarball，并由 `package-lock.json` 的 resolved 与
SHA-512 integrity 复核。任何其他写法都明确失败，不存在“未识别所以跳过”的成功路径。

若检查报 unknown package/member，不要把未知项加入 allowlist 让测试变绿；先确认它是否经过共享准入。若
报旧名称，修改真实产品身份，而不是用字符串拼接绕过扫描；policy 自己为了定义拒绝项而拼接是有意避免
自命中。

### 6.1 当前密码散列身份

| 参数 | 当前值 |
|---|---:|
| algorithm | Argon2id |
| PHC version | `v=19` |
| memory | `19456 KiB` |
| iterations | `2` |
| parallelism | `1` |
| salt | `16 bytes` |
| output | `32 bytes` |
| plaintext length | `12..1024 bytes`，无 ASCII control |

`require_current_password_hash` 会重新解析并要求 PHC string 自身是 canonical，随后精确检查上述参数。任何
参数差异都不是“仍然安全所以可接受”，而是 current contract mismatch。产品发现持久 hash 不符合时应
启动失败，需要操作者核对当前凭据；在线登录不能尝试第二套 verifier。

### 6.2 Username 与 token

- 管理员登录 username 候选必须是 1～64 个 printable ASCII bytes；先 `trim_ascii`、再 ASCII lowercase。
  control 在 trim 前已拒绝，所以实际只移除两端 U+0020 space；候选 guard 通过不表示身份已接受。
- canonical username 必须是 3～64 bytes，首尾为字母/数字，全部字符只来自 `[a-z0-9._-]`；`@`、Unicode、
  control character 和其他符号拒绝。相邻分隔符允许，xcss 不赋予点号任何域名语义。
- 持久状态和 Session 必须已经 canonical，产品启动时验证但不能悄悄改写；数据库应同时建立同义 CHECK。
- Session/CSRF token 是 32-byte OS 随机数，经 URL-safe Base64 无 padding 编为 43 字符。
- `token_hash`/`token_hash_hex` 是 SHA-256 摘要工具；只有先通过 token shape 才允许做 Session 匹配。
- `token_matches_hash` 和 CSRF helper 使用 constant-time 比较，expected digest 长度必须为 32 bytes。

随机源失败必须中止当前 Session 创建，不回退到时间、UUID、伪随机数或可预测 counter。

### 6.3 同源故障表

| 现象 | 权威解释 | 运维动作 | 禁止做法 |
|---|---|---|---|
| Missing Origin/Host/Sec-Fetch-Site | 请求未满足当前浏览器管理面合同 | 检查真实浏览器/代理是否保留 header | 在 Server 增加 missing fallback |
| Duplicate header | 代理或攻击请求形成歧义 | 保留原始请求证据，修正代理 | 选择第一/最后一个值 |
| OriginHostMismatch | scheme/host/effective port 不一致 | 校正外部 URL、Host forwarding 和监听模式 | 信任任意 X-Forwarded-Host |
| UnexpectedOriginScheme | production 收到 HTTP 或 dev 收到 HTTPS | 修正模式/TLS 终止 | 同时接受 HTTP/HTTPS |
| DevelopmentHostIsNotLoopback | HTTP dev 请求指向非 loopback | 使用 HTTPS production 或真实 loopback | 添加私网段例外 |
| InvalidCsrfToken | header 非唯一 43 字符 token | 检查当前 Web build/Session | 接受空值、短 token 或 Cookie 代替 |
| CsrfTokenMismatch | token 不属于当前 Session | 重新建立当前 Session并查竞态/代理缓存 | 自动尝试其他 Session |

### 7.1 四阶段

1. `pnpm typecheck` 先构建根 dist，再验证所有内部模块源码类型；
2. `pnpm test` 先 clean/build 根 dist，再从 `dist` 运行单元/契约测试；
3. `package-artifacts.py check` 只审计现有 dist；
4. `package-artifacts.py smoke` clean、重建、pack、检查 tar并在空目录 isolated install + TypeScript/Vite。

只有第 4 阶段证明实际发布形态。monorepo 软链接能掩盖缺失 dependency/export，所以不能只执行第 1、2
阶段后发布。

### 7.2 Package 内容规则

- package 必须是 ESM，`files` 只含 `dist`，公开入口全由 `exports` 声明；
- manifest、export 目标和静态文件必须是普通单链接文件；
- `dist` 必须是真实目录且每次构建前清空；
- tar member 必须 canonical、无 absolute/`..`/backslash、duplicate、symlink、hardlink、device 或源码泄漏；
- 发布 manifest 内不得出现 `workspace:`；
- 内部模块以 `@xcss/web/<module>` 自引用；外部 React/Vite peer 使用固定精确版本，消费者显式拥有；
- 临时消费者使用 `npm --ignore-scripts` 安装唯一真实 tgz 并解析所有 export。

### 7.3 常见故障

| 输出/现象 | 根因 | 正确处理 |
|---|---|---|
| `ERR_PNPM_OUTDATED_LOCKFILE` | manifest 与 lock 不一致 | 用固定 pnpm 审查并重建 lock；不要删除 lock |
| engine warning/failure | Node 不是 26.7.0 | 切换 Node；不要放宽 `<27`/最低版本 |
| export missing | build/copy script 与 manifest 分叉 | 修复 source→dist 和 export 单一事实源 |
| stale artifact | clean 没清除被删产物 | 修复 clean，重跑 smoke；不要手工补文件 |
| tar contains workspace | runtime dependency 声明错误 | 删除内部 package 依赖，使用同包子路径；外部 peer 使用精确版本 |
| peer resolution failure | 没同时安装显式 peer | 修正消费者依赖或 package metadata |
| linked file rejected | package 树含 symlink/hardlink | 生成真实单链接文件；查供应链污染 |
| admin toolchain assertion | 产品 React/Vite/Node 漂移 | 全产品同步使用精确 xcss baseline |

### 7.4 Server 与 Web 的统一构建

从 xcss `0.10.8` 起，带 Web 的 Server component 必须声明 `embedded-web` capability，并提供
`xcss-web-build.json`。`xcss-build-server --mode release` 依次执行 npm 锁定安装、前端构建、
规范目标 Rust 编译及实际二进制的 `web-assets` 验收；构建命令来自发布包 `@xcss/web/web-toolchain`。
Web 输出路径通过 `XCSS_WEB_DIST` 传给 Vite/native preset 和 Rust build script，避免各产品各自猜测
目录和先后顺序。Rust package 在 runtime 和 build-dependencies 中都使用同一精确版本的
`xcss::web_assets`。

二进制中的清单绑定每个资源的路径、MIME、大小和 SHA-256。发行树可以携带这份清单并与二进制逐字节
核对，生产请求直接使用内嵌字节。不要再次发行 raw Web 目录来保存重复副本，也不要用可重写的外部
清单授权二进制之外的资源。产品的签名、伴随进程、安装权限、部署位置和状态合同仍由产品验收。
拥有已验证源码归档或专用离线编译流程的扩展可以调用 `--verify-only --binary <path> --dist <path>`，
共享验收不会跳过实际可执行文件；扩展自身继续证明其源码身份与正式目标。

开发时运行 `xcss-build-server --mode development`。需要热更新时明确选择安全目录提供器或 Vite
开发服务，后续 Web 修改不要求重新编译 Rust。需要验收编译产物时选择内嵌提供器。正式、源码绑定的
可执行文件必须拒绝开发目录覆盖；不得因目录缺失而静默回退到旧资源。

`xcss-conformance verify-source` 与 `verify-web` 检查 capability、构建声明、仓库内规范路径、npm
script 与共同 crate 依赖。历史版本按历史声明接受检查，报告不会把它们解释为已采用新构建合同。
完整接入步骤见 [统一 Server/Web 构建指南](beginner-guide/11-embedded-web-build.md)。

## 8. Rust crate 消费与排障

xcss 当前不要求 crates.io 在线依赖。正式消费者使用 release tag 对应完整 commit：

```toml
xcss = {
  git = "https://github.com/isarmg/xcss.git",
  rev = "<v1.0.0 对应的 40 位 commit>",
  version = "=1.0.0"
}
```

不得使用 branch、短 SHA、浮动 tag 或永久 sibling path。Cargo 只声明一次 `xcss`，Rust代码按需求使用内部模块：

| 需求 | Rust 模块 |
|---|---|
| username/密码/token/same-origin | `xcss::admin_auth` |
| wire contract | `xcss::contracts` |
| Error Envelope | `xcss::error` |
| Schema 身份算法 | `xcss::schema_identity` |
| Server target 常量与检查 | `xcss::server_target` |
| SQLx SQLite 服务 | `xcss::sqlite` |

这些模块共享一个编译、版本、来源及许可证身份，不再各自发布。

| 故障 | 解释与处理 |
|---|---|
| `links=sqlite3` 冲突 | 同一依赖图出现不兼容的 SQLite 链接身份；检查产品依赖图，统一为当前 SQLx adapter 对应的 SQLite 来源 |
| Server 交叉 target compile_error | 当前 Server 只支持 GNU/Linux AMD64；不要绕过 gate |
| `open_existing` missing | 路径或部署错误；不要改成隐式 create |
| product_metadata mismatch | DDL/列/storage class 非当前合同；停止并定位来源 |
| identity mismatch | product/version/revision/hash非当前状态；拒绝运行并保全输入 |
| fingerprint mismatch | metadata 声明与实际 DDL 不同；不能只覆盖 metadata |
| checkpoint busy/incomplete | 仍有 reader/writer；进入产品定义的安全维护窗口后重试 |
| integrity/FK violation | 数据库不可信；阻止运行/备份，保全现场并按产品流程处置 |

## 9. Consumer Matrix 运维

`consumers/consumer-matrix.json` 是追踪证据，不是发布宣传。字段解释：

| 字段 | 运维含义 |
|---|---|
| `product` | 6 个真实产品仓库之一的产品标识 |
| `commit` | 本次评估采用的已提交消费者基线，必须是完整 SHA |
| `xcss_version` | 该提交采用的 xcss 版本；未集成为 null |
| `packages` | 直接采用的组件，不列传递依赖 |
| `status` | not-migrated / migration-in-progress / conforming / non-conforming / temporary-exception |
| `exceptions` | 非 conforming 状态对应的显式例外编号；conforming 必须为空 |

发布前本地 path/file 联调最多标 `migration-in-progress`；xcss release 后，将消费者换成 Git rev/tgz、
重建 lock、完整验证并提交，才能标 `conforming`。若 CI 后来失败，应真实标 `non-conforming`；存在有效迁移
例外时标 `temporary-exception`，不能保留过期绿色状态。

## 10. CI 与供应链策略

`scripts/check-workflow-supply-chain.py` 对 `.github/workflows/*.yml` 实施 fail-closed 文本 policy：

- runner 固定 `ubuntu-24.04`，job timeout 必须 1～30 分钟；
- 顶层 `permissions: {}`；普通 job 只允许 `contents: read`；
- action 只接受 allowlist 中的完整 40 位 commit SHA；
- checkout 必须 `persist-credentials: false`；
- setup-node 必须 `26.7.0` 且 `check-latest: false`；
- 禁止 YAML anchor、alias、merge key及在伪造结构中出现 action；
- 唯一 `contents: write` 例外是精确路径 `release.yml` 的唯一 `release` job；
- release workflow 只能由 `push.tags: v*` 触发。

policy 的 invalid fixture 覆盖写权限、浮动 action/runner/Node、缺 checkout/timeout、persisted credential、
YAML anchor 和 action outside steps。修改 workflow policy 时必须同时新增能证明 fail-closed 的负例。

### 11.1 前置条件

- `main` 已推送且 source tree 无 tracked/untracked 文件；
- 全部质量门通过；
- 版本与工具链事实源一致；
- 至少一个会实际触发本次改动的真实消费者完成发布前联调；若改动跨语言 wire、认证、Schema 算法或 Web
  runtime，必须覆盖至少两个不同产品，不能用 xcss 自测替代消费者证据；
- 单个 Rust crate 的真实 Cargo package 清单均携带审核过的根 `LICENSE`；
- GitHub 不存在同名 tag/release；
- tag `v1.0.0` 精确指向当前 HEAD，source revision 为完整小写 SHA。

### 11.2 构建命令与输出

release workflow 调用：

```bash
python3 scripts/build-release-assets.py \
  --output "$RUNNER_TEMP/xcss-release" \
  --source-revision "$GITHUB_SHA" \
  --tag "$GITHUB_REF_NAME"
```

```text
xcss-release/
├─ release-tree.json
└─ artifacts/
   ├─ xcss-web-1.0.0.tgz
   ├─ xcss-release-tool-1.0.0.tar.gz
   ├─ state-contract.json
   ├─ release-identity.json
   ├─ build-inventory.json
   └─ SHA256SUMS
```

xcss state contract 的 `schema=null`，lock/resource/external/companion 数组为空，因为本仓无运行时状态。
release identity 恰好五字段并用 `state_contract_sha256` 绑定它。tool bundle 固定 mtime/owner/group/mode和
排序；inventory 描述精确 toolchain、两个 lockfile hash、单个 Rust crate、单个 npm package 和已生成资产。

### 11.3 Release-tree 防护

- 路径拒绝 absolute、`..`、backslash、NUL 和超长值；
- 输入/验证对象只允许普通单链接文件，不接受 symlink/hardlink/special file；
- 文件/tree/count/manifest/总大小有界；
- no-follow file descriptor 计算 hash，读取后复核 inode/size/time/path identity；
- manifest 记录 exact path、mode、size、SHA-256，并拒绝额外或缺失文件；
- manifest 位于被描述 artifacts 树外，不递归描述自身。

## 12. 发布后复核

1. 从 GitHub Release 下载所有资产到空目录；
2. 在 `artifacts/` 内运行 `sha256sum -c SHA256SUMS`；
3. 使用随附 release tool 验证 `artifacts/` 与外层 `release-tree.json`；
4. 分别检查 全部包的真实 tgz 的 package name/version/exports，执行隔离目录正常 peer 安装；
5. 将每个消费者的 Rust path 换为完整 Git rev、Web file 换为 release tgz URL；
6. 重建消费者 lock，在独立 checkout 完成完整产品测试、发行解包和断网运行；
7. 按产品提交并更新 consumer matrix 的真实 commit/status。

发布失败时不得移动 tag、覆盖 asset、删除 release 后重建同版本。修复源码，使用新的唯一当前版本发布。

## 13. HTTP client 故障语义

| `ApiClientError.code` / 现象 | 精确含义 | 调用方动作 |
|---|---|---|
| `network_error` | fetch 未获得权威 HTTP response | 显示网络状态；按业务幂等性决定是否重试 |
| `request_timeout` | client deadline 先触发；服务端可能已执行 | mutation 标记结果未知，不自动重复 |
| `request_aborted` | 调用方取消先触发；服务端仍可能执行 | 根据 operation/report ID 查询权威状态 |
| `response_too_large` | 声明或实际字节超过预算 | 查服务异常/选择专用 streaming API，不提高到无界 |
| `invalid_content_type` | 成功 response 不是 JSON/`+json` | 查反向代理、错误页和 Server contract |
| `invalid_json_response` | UTF-8/JSON 解析失败 | 保留 request ID，查 Server/代理，不展示 raw body |
| `invalid_error_response` | 非 2xx 未返回严格 Error Envelope | 使用安全通用文案，不回显 HTML/Secret |
| 401 | 当前 Session 无效 | admin-web 在仍匹配发出时 Session 时清理本地状态 |
| 无效 Retry-After | header 不符合整数/规范 HTTP-date或溢出 | 忽略 hint，由产品策略决定 |

401 cleanup callback 抛错不能覆盖权威 API error。`Retry-After` 最多 24 小时，只是提示，不触发自动 retry。

## 14. 依赖更新

每次依赖更新单独提交并记录：上游源码/公告、license、启用 feature/default、Rust MSRV/Node engine、native
dependency、bundle/compile size、API 行为和消费者影响。更新顺序：xcss manifest/lock → xcss
全门禁 → package tarball → consumer matrix 中所有采用者 → 独立产品 release 验证。若消费者仍调用被删
API，应同步升级消费者；不得在 xcss 添加 alias 维持另一代。

### 15.1 通用处置

1. 暂停 tag/package/release，撤销或轮换受影响的 GitHub/npm credential；
2. 保全 workflow run、commit/tag、release metadata、asset digest、lockfile、consumer matrix和审计日志；
3. 确定漏洞组件、可达调用和所有消费者 commit；
4. 在唯一当前源码中修复，完整验证并发布新不可变版本；
5. 所有消费者更新精确依赖并重建产品制品；
6. 更新 matrix 和事件记录，确认未泄露生产 Secret。

### 15.2 认证 primitive 事件

若问题涉及 Argon2 policy、随机 token、same-origin/CSRF 或管理员合同，应同时审计全部消费者的：启动时
持久凭据验证、登录限流、Cookie flags、Session TTL/撤销、全部原始 header 收集、HTTP2 authority、Web
内存 Session 和 stale-response 竞态。xcss 修复库并不自动修复已编译产品，必须逐产品发布。

### 15.3 供应链事件

若 tag、action、package 或 asset 可能被替换，先比较 Git object、tag object、release asset SHA、
`SHA256SUMS`、release-tree、build inventory 与 registry provenance。不要删除证据或覆盖原资产；使用新版本
恢复信任链。

## 16. 备份、保留与定期审计

需要备份：Git 仓库及对象、annotated tag、GitHub Release metadata/assets、CI 配置、Cargo/pnpm lock、
Schema/fixture、consumer matrix 和文档。registry cache、`node_modules`、`target`、`dist` 不是源码备份。

建议每个发布周期至少执行：从空缓存 locked install；全部包的真实 tgz 离线安装；release-tree 回下载验证；所有
consumer matrix 项状态复核；完整 SHA action 与权限扫描；旧名称/current-only 扫描；管理员合同/Server
target 跨产品抽查。xcss 无业务数据，所以不得把产品 backup 文件复制进本仓或 Release。

## 当前认证与发布验收

账户保持既有 DDL、ID 和 Argon2 PHC。完成 bootstrap 后必须通过单活动管理员只读校验；多账户、非活动账户或非法记录应停止启动，运维显式处理，校验不自动改写数据。
CSRF 由同会话 Token 稳定派生；不符合派生值的会话被拒绝，需要重新登录。Web 退出未确认时使用“重试退出”，关闭浏览器不保证服务端撤销。
登录失败阈值限制新请求准入；阈值前已准入的有限请求可完成，仍受 Argon2 槽位与失败记录容器上限约束。
Release 在同一提交构建后执行 conformance、Chromium/Firefox 浏览器验收，再构建发布树；失败必须阻止发布。
包 smoke 的依赖准备允许联网，使用 manifest 精确 peers 与真实 tgz，在隔离目录执行 Node 导入、TypeScript 和 Vite JS/CSS 构建。运行时无注册表依赖。

本次 1.0.0 重建已清空旧历史的 consumer 审计快照，初始登记表为空。后续登记需填入实际核验的提交与依赖版本，不复用旧报告或宣称尚未验证的 conforming。


## 17. 固定数据接口调整后的重新部署

1.0.0 使用[当前中立数据接口](configuration-cli-logging.md#当前固定数据接口)。程序只处理当前名称和当前
schema，不提供旧字段、旧锁名、旧表名的兼容入口，也不自动迁移历史数据库。服务尚未部署时，直接按产品
安装文档创建新的空状态目录并初始化即可。已部署服务按下面步骤离线处理。

### 17.1 确认目标并停服

先从对应产品文档和服务配置确认服务单元、状态目录及运行账户。下面变量必须填写为该产品的实际值；
备份目录使用本次操作独有且尚未存在的绝对路径。

```bash
SERVICE_UNIT='填写实际服务单元，例如 product.service'
STATE_DIR='填写实际状态目录的绝对路径'
BACKUP_DIR='填写新备份目录的绝对路径'
systemctl cat "$SERVICE_UNIT"
sudo systemctl stop "$SERVICE_UNIT"
systemctl is-active "$SERVICE_UNIT"
```

`systemctl cat` 展示实际启动命令、配置和覆盖项，用来核对操作对象。`stop` 正常停止服务；`is-active`
应显示 `inactive` 或 `failed`，非零退出码在这个确认步骤是停服结果。还需关闭产品的离线维护工具、计划任务
和其他写入进程，并确认服务不会自动重新启动。状态锁的新旧名称不同，不能让两种版本同时访问同一状态。

### 17.2 保存完整离线备份

```bash
sudo install -d -m 0700 -- "$BACKUP_DIR"
sudo tar --acls --xattrs --numeric-owner -cpf "$BACKUP_DIR/state-before-redeploy.tar" \
  -C "$(dirname -- "$STATE_DIR")" "$(basename -- "$STATE_DIR")"
sudo sha256sum "$BACKUP_DIR/state-before-redeploy.tar"
sudo tar -tf "$BACKUP_DIR/state-before-redeploy.tar"
```

`install -d -m 0700` 创建仅管理员可读的备份目录；操作前确认该路径不存在。`tar` 以原 UID/GID、权限、ACL
和扩展属性保存整个已停写状态目录，包括 SQLite 主库及仍存在的 WAL、journal、SHM 和产品配置。
`sha256sum` 给备份生成完整性摘要，记录在维护记录中；`tar -tf` 只列出归档内容，用来核对目录与必要文件齐全。
产品媒体文件等若位于状态目录之外，应按产品文档单独备份。凭据和用户数据随备份保密存放。

### 17.3 在新目录重新初始化

保留原状态目录和备份，不在原库上批量执行表名替换。按对应产品部署文档使用新的空私有状态目录，
配置相同的服务运行账户，再执行该产品当前的初始化命令。初始化由当前程序创建 0700 目录、0600 文件、
当前锁文件以及当前 schema；不要通过放宽 owner、mode、链接检查或手填 metadata 来让旧库通过启动。

SQLite 指纹算法版本仍为 1：按 SQLite 的 BINARY 顺序读取 `(type,name,tbl_name,sql)`，排除 `sqlite_*`
及 `product_metadata`，每个 UTF-8 字段以 u64 大端长度分帧后计算 SHA-256。公共表、索引及外键目标的名称
变化会改变指纹；`ALTER TABLE` 还可能重写 DDL 的引号和原始文本。仅重命名表或替换
`product_metadata.schema_sha256` 都不能保证库符合编译时的当前 schema。当前四分量身份、严格平台 DDL、
存储类型、CHECK、外键和索引验证都继续生效。

### 17.4 人工转移产品数据并重新建立管理会话

先检查产品是否提供当前版本的导出/导入操作；按对应产品文档在离线环境转移业务内容到新 schema，
核对数据数量、文件摘要、外键和产品业务约束。没有经过验证的导入工具时，保留原始备份并重新部署所需
配置与业务内容，不直接整库覆盖新状态。管理员按当前初始化流程设置，浏览器重新登录建立新 Cookie；
无需沿用原管理会话或手工复制 Session/CSRF token。

### 17.5 验证并启服

使用产品现有的只读配置校验、数据库身份校验和诊断命令，检查当前 application、application_version、
schema_revision、schema_sha256 以及实际现场指纹。确认配置指向新状态目录、产品的发行身份和 Web
资源清单摘要均与本次发布一致，然后启服：

```bash
sudo systemctl start "$SERVICE_UNIT"
systemctl status "$SERVICE_UNIT" --no-pager
sudo journalctl -u "$SERVICE_UNIT" -n 100 --no-pager
```

`start` 启动已经完成初始化和校验的当前版本；`status` 显示运行状态与启动结果；`journalctl` 查看该单元
最近 100 条日志，确认 readiness、认证及业务任务无异常。继续运行产品文档中的实际状态查询和业务验收。
若验证失败，停服后保存诊断输出并修复具体原因；原状态与备份在本次验收完成并达到产品保留期前保留。
