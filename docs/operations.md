# Sarmg Foundation 运维文档

## 1. 运维范围

Foundation 没有生产 daemon、监听端口、业务数据库、用户表、Session 表、systemd unit 或运行时配置文件。
本文件中的“运维”专指源码仓库、固定工具链、依赖锁、CI 权限、npm package、release asset、消费者采用
证据和共享安全事件。产品数据库备份、恢复、迁移、Secret 轮换和服务启停应查各产品及 `sarmg-upgrade`
的运维文档。

## 2. 当前身份与事实源

| 项目 | 唯一当前值 | 权威位置 | 漂移时的处理 |
|---|---|---|---|
| Foundation 版本 | `0.4.0` | 根 `Cargo.toml`、`package.json`、各 crate/package、policy | 阻止 CI/发布，统一更新后重建 lock |
| Rust | `1.98.0` | `rust-toolchain.toml` | 不用其他版本代替验证 |
| Rust edition/MSRV | 2024 / `1.98` | workspace package | 作为工具链大问题单独升级 |
| Node | `26.7.0` | `.node-version`、`engines.node`、CI | 切换 Node，不放宽 engine |
| pnpm | `10.12.1` | 根 `packageManager`、CI | 安装精确版本，不使用 Corepack 浮动解析 |
| TypeScript | `5.8.3` | 4 个 package manifest、lock | 与产品 Web 基线一起升级 |
| React / React DOM | `19.2.8` | `admin-web` toolchain/peer/dev deps | 所有非 Dufs Web 同步验证 |
| Vite / React plugin | `7.3.6` / `4.7.0` | `admin-web` toolchain/peer/dev deps | 所有非 Dufs Web 同步验证 |
| Server target | `x86_64-unknown-linux-gnu` | `sarmg-server-target` | Server 其他 target 编译必须失败 |
| License | Apache-2.0 | 根及六个 crate 的 `LICENSE`、Cargo/npm metadata、Cargo package 清单 | 缺失或字节漂移即不发布 |
| Release tag | `v0.4.0` | Git tag | 一经发布不移动、不覆盖、不重建同版本 |

Foundation 自身的 source/tool release identity 默认 target 是 `source-any`；`sarmg-server-target` 是消费者
Server 的编译门禁，不能把 Foundation 误写成 AMD64 在线服务。

## 3. 工作站准备

### 3.1 必需工具

```bash
rustup toolchain install 1.98.0 --profile minimal --component rustfmt,clippy
node --version
pnpm --version
python3 --version
git --version
```

Node 输出必须为 `v26.7.0`，pnpm 必须为 `10.12.1`。Python 与 Git 没有在本仓声明一个可发布 runtime，
但必须支持当前脚本和平台。首次安装依赖：

```bash
pnpm install --frozen-lockfile --ignore-scripts
cargo metadata --locked --no-deps --format-version 1
```

不要把 `cargo update`、`pnpm update`、删除 lockfile 或无 `--frozen-lockfile` 安装作为普通排障方式。依赖
更新需要独立评审，不应通过重新解析隐藏 manifest/lock 漂移。

### 3.2 可安全删除的缓存

在确认目标是本仓具体目录后，可以删除并重建 `target/`、根 `node_modules/`、各 package `dist/` 和临时
release 输出。不可将 `Cargo.lock`、`pnpm-lock.yaml`、Schema、fixture、consumer matrix、文档或 Git tag
当缓存处理。不要对工作区根或未解析变量使用递归删除。

## 4. 统一质量门

代码与文档全部完成后，按顺序执行：

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
git status --short
```

失败后修复实际原因，从受影响层向下重跑；最终交付前再完整跑一次。禁止用以下方式“修复”失败：放宽
strict guard、接受另一套 Argon2 参数、忽略重复安全 header、提高到无界 body、删除竞态/攻击负例、添加
旧字段 alias、跳过 tarball offline install 或把消费者矩阵手改为 conforming。

## 5. Repository Policy 运维

`scripts/check-foundation.py` 调用 `tools/foundation_policy.py`，当前核对：

- Cargo/npm/policy 的版本均为 `0.4.0`；
- Rust `1.98.0`、Node `26.7.0`、pnpm `10.12.1` 的事实源一致；
- Rust workspace 恰好包含 6 个已知 crate，npm workspace 恰好包含 4 个已知 package；
- 根 `LICENSE` 必须匹配审核过的 Apache-2.0 SHA-256；六个 crate 必须各有普通、单链接、byte-exact副本，
  且 `cargo package --list` 必须把它作为唯一根 `LICENSE` 分发；
- 所有 crate 启用 workspace lint，内部 Rust dependency 精确 `=0.4.0`；
- 内部 npm build dependency 使用 `workspace:0.4.0`，peer 使用精确 `0.4.0`；
- `admin-web` 的 React/Vite/TypeScript/type package 精确一致；
- consumer matrix 只含 6 个已知消费者、已知组件和自洽状态；
- 源码和文档不存在已取消的项目/客户端名称。

若检查报 unknown package/member，不要把未知项加入 allowlist 让测试变绿；先确认它是否经过共享准入。若
报旧名称，修改真实产品身份，而不是用字符串拼接绕过扫描；policy 自己为了定义拒绝项而拼接是有意避免
自命中。

## 6. 认证共享原语运维

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
启动失败，并通过独立管理/升级动作重新建立当前凭据；在线登录不能尝试第二套 verifier。

### 6.2 Username 与 token

- 管理员登录 username 候选必须是 1～64 个 printable ASCII bytes；先 `trim_ascii`、再 ASCII lowercase。
  control 在 trim 前已拒绝，所以实际只移除两端 U+0020 space；候选 guard 通过不表示身份已接受。
- canonical username 必须是 3～64 bytes，首尾为字母/数字，全部字符只来自 `[a-z0-9._-]`；`@`、Unicode、
  control character 和其他符号拒绝。相邻分隔符允许，Foundation 不赋予点号任何域名语义。
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

## 7. Web package 构建与排障

### 7.1 四阶段

1. `pnpm typecheck` 验证源码类型，不写产物；
2. `pnpm test` 由各包先 clean/build，再从 `dist` 运行单元/契约测试；
3. `package-artifacts.py check` 只审计现有 dist；
4. `package-artifacts.py smoke` clean、重建、pack、检查 tar并在空目录 offline install。

只有第 4 阶段证明实际发布形态。monorepo 软链接能掩盖缺失 dependency/export，所以不能只执行第 1、2
阶段后发布。

### 7.2 Package 内容规则

- package 必须是 ESM，`files` 只含 `dist`，公开入口全由 `exports` 声明；
- manifest、export 目标和静态文件必须是普通单链接文件；
- `dist` 必须是真实目录且每次构建前清空；
- tar member 必须 canonical、无 absolute/`..`/backslash、duplicate、symlink、hardlink、device 或源码泄漏；
- 发布 manifest 内不得出现 `workspace:`；
- `admin-web`/`http-client` 的内部 peer 必须精确版本，消费者显式拥有；
- 临时消费者使用 `npm --offline --ignore-scripts` 同时安装 4 个 tgz 并解析所有 export。

### 7.3 常见故障

| 输出/现象 | 根因 | 正确处理 |
|---|---|---|
| `ERR_PNPM_OUTDATED_LOCKFILE` | manifest 与 lock 不一致 | 用固定 pnpm 审查并重建 lock；不要删除 lock |
| engine warning/failure | Node 不是 26.7.0 | 切换 Node；不要放宽 `<27`/最低版本 |
| export missing | build/copy script 与 manifest 分叉 | 修复 source→dist 和 export 单一事实源 |
| stale artifact | clean 没清除被删产物 | 修复 clean，重跑 smoke；不要手工补文件 |
| tar contains workspace | runtime dependency 声明错误 | peer 用精确版本，workspace 仅 dev/build 使用 |
| offline peer failure | 没同时安装显式 peer | 修正消费者依赖或 package metadata |
| linked file rejected | package 树含 symlink/hardlink | 生成真实单链接文件；查供应链污染 |
| admin toolchain assertion | 产品 React/Vite/Node 漂移 | 全产品同步使用精确 Foundation baseline |

## 8. Rust crate 消费与排障

Foundation 当前不要求 crates.io 在线依赖。正式消费者使用 release tag 对应完整 commit：

```toml
sarmg-admin-auth = {
  git = "https://github.com/isarmg/sarmg-foundation.git",
  rev = "<v0.4.0 对应的 40 位 commit>",
  version = "=0.4.0"
}
```

不得使用 branch、短 SHA、浮动 tag 或永久 sibling path。按最小需要选择：

| 需求 | 应选 crate | 不应额外引入 |
|---|---|---|
| username/密码/token/same-origin | `sarmg-admin-auth` | SQLx、Web package |
| wire contract | `sarmg-contracts` | 产品 DTO框架 |
| 仅 Error Envelope | `sarmg-error` | 完整 contracts |
| rusqlite/离线 Schema 算法 | `sarmg-schema-identity` | `sarmg-sqlite` |
| Server target gate | `sarmg-server-target`（只给 Server crate） | 客户端/Agent crate |
| SQLx SQLite 服务 | `sarmg-sqlite` | 产品 migration/backup 假设 |

| 故障 | 解释与处理 |
|---|---|
| `links=sqlite3` 冲突 | 错误地把 SQLx adapter 引入 rusqlite 工具；改用纯 identity crate |
| Server 交叉 target compile_error | 当前 Server 只支持 GNU/Linux AMD64；不要绕过 gate |
| `open_existing` missing | 路径或部署错误；不要改成隐式 create |
| product_metadata mismatch | DDL/列/storage class 非当前合同；停止并定位来源 |
| identity mismatch | product/version/revision/hash 非当前状态；交给独立升级/重建流程 |
| fingerprint mismatch | metadata 声明与实际 DDL 不同；不能只覆盖 metadata |
| checkpoint busy/incomplete | 仍有 reader/writer；进入产品定义的安全维护窗口后重试 |
| integrity/FK violation | 数据库不可信；阻止运行/备份，保全现场并按产品流程处置 |

## 9. Consumer Matrix 运维

`consumers/consumer-matrix.json` 是追踪证据，不是发布宣传。字段解释：

| 字段 | 运维含义 |
|---|---|
| `repository` | 6 个真实产品仓库之一 |
| `commit` | 本次评估采用前/联调基线，必须完整 SHA |
| `adopted_version` | 当前采用的 Foundation 版本；未集成为 null |
| `packages` | 直接采用的组件，不列传递依赖 |
| `status` | not-migrated / migration-in-progress / conforming / non-conforming / temporary-exception |
| `last_verified_commit` | 不可变来源、独立 checkout 全部通过的最终消费者 SHA |

发布前本地 path/file 联调最多标 `migration-in-progress`；Foundation release 后，将消费者换成 Git rev/tgz、
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

## 11. Release 构建

### 11.1 前置条件

- `main` 已推送且 source tree 无 tracked/untracked 文件；
- 全部质量门通过；
- 版本与工具链事实源一致；
- 至少一个会实际触发本次改动的真实消费者完成发布前联调；若改动跨语言 wire、认证、Schema 算法或 Web
  runtime，必须覆盖至少两个不同产品，不能用 Foundation 自测替代消费者证据；
- 十三个 Rust crate 的真实 Cargo package 清单均携带审核过的根 `LICENSE`；
- GitHub 不存在同名 tag/release；
- tag `v0.4.0` 精确指向当前 HEAD，source revision 为完整小写 SHA。

### 11.2 构建命令与输出

release workflow 调用：

```bash
python3 scripts/build-release-assets.py \
  --output "$RUNNER_TEMP/sarmg-foundation-release" \
  --source-revision "$GITHUB_SHA" \
  --tag "$GITHUB_REF_NAME"
```

```text
sarmg-foundation-release/
├─ release-tree.json
└─ artifacts/
   ├─ sarmg-admin-web-0.4.0.tgz
   ├─ sarmg-contracts-0.4.0.tgz
   ├─ sarmg-design-tokens-0.4.0.tgz
   ├─ sarmg-http-client-0.4.0.tgz
   ├─ sarmg-release-tool-0.4.0.tar.gz
   ├─ state-contract.json
   ├─ release-identity.json
   ├─ build-inventory.json
   └─ SHA256SUMS
```

Foundation state contract 的 `schema=null`，lock/resource/external/companion 数组为空，因为本仓无运行时状态。
release identity 恰好五字段并用 `state_contract_sha256` 绑定它。tool bundle 固定 mtime/owner/group/mode和
排序；inventory 描述精确 toolchain、两个 lockfile hash、13 个 crate、4 个 package 和已生成资产。

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
4. 分别检查 4 个 tgz 的 package name/version/exports，执行空目录离线安装；
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
dependency、bundle/compile size、API 行为和消费者影响。更新顺序：Foundation manifest/lock → Foundation
全门禁 → package tarball → consumer matrix 中所有采用者 → 独立产品 release 验证。若消费者仍调用被删
API，应同步升级消费者；不得在 Foundation 添加 alias 维持另一代。

## 15. 安全事件

### 15.1 通用处置

1. 暂停 tag/package/release，撤销或轮换受影响的 GitHub/npm credential；
2. 保全 workflow run、commit/tag、release metadata、asset digest、lockfile、consumer matrix和审计日志；
3. 确定漏洞组件、可达调用和所有消费者 commit；
4. 在唯一当前源码中修复，完整验证并发布新不可变版本；
5. 所有消费者更新精确依赖并重建产品制品；
6. 更新 matrix 和事件记录，确认未泄露生产 Secret。

### 15.2 认证 primitive 事件

若问题涉及 Argon2 policy、随机 token、same-origin/CSRF 或管理员合同，应同时审计 6 个消费者的：启动时
持久凭据验证、登录限流、Cookie flags、Session TTL/撤销、全部原始 header 收集、HTTP2 authority、Web
内存 Session 和 stale-response 竞态。Foundation 修复库并不自动修复已编译产品，必须逐产品发布。

### 15.3 供应链事件

若 tag、action、package 或 asset 可能被替换，先比较 Git object、tag object、release asset SHA、
`SHA256SUMS`、release-tree、build inventory 与 registry provenance。不要删除证据或覆盖原资产；使用新版本
恢复信任链。

## 16. 备份、保留与定期审计

需要备份：Git 仓库及对象、annotated tag、GitHub Release metadata/assets、CI 配置、Cargo/pnpm lock、
Schema/fixture、consumer matrix 和文档。registry cache、`node_modules`、`target`、`dist` 不是源码备份。

建议每个发布周期至少执行：从空缓存 locked install；4 个 tgz 离线安装；release-tree 回下载验证；所有
consumer matrix 项状态复核；完整 SHA action 与权限扫描；旧名称/current-only 扫描；管理员合同/Server
target 跨产品抽查。Foundation 无业务数据，所以不得把产品 backup 文件复制进本仓或 Release。
