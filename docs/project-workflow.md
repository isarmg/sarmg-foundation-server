# Sarmg Foundation 工作流程与流程树

## 1. 文档目标

本文描述一项能力从产品本地需求进入 Foundation、形成可发布合同、被真实产品采用并最终受运维约束的
完整流程。Foundation 是所有产品的上游平台规范和安全下限；一次看似很小的 helper 修改可能同时改变 Server、Web、
SQLite、离线工具和发布树，所以“源码编译通过”只是中间节点，不是完成定义。

本仓坚持 current-only：每个版本只存在一套公开名称、格式和行为。任何历史读取、字段 alias、双写、旧
散列降级或运行时版本分支都不进入该流程；确需处理已发布状态时，由 `sarmg-upgrade` 建立独立、精确、
离线的转换流程。

## 2. 全局流程树

```text
提出能力
├─ 是否属于平台责任？
│  └─ 是：Foundation 定义唯一规范和实现
├─ 是否属于已知运行形态差异？
│  └─ 是：选择或新增产品无关 Profile/Capability
├─ 是否属于产品业务语义？
│  └─ 是：产品通过 Adapter/Trait 实现
└─ 是否属于历史持久格式？
   └─ 是：只在 sarmg-upgrade 中建立离线转换边

确定所有权
├─ 收集实际调用点、最强安全约束和删除后果
├─ 写唯一当前 API、输入上限、错误、并发和所有权
├─ 先同步实现、正例/负例、Schema/fixture、中文文档
├─ 运行 Foundation 全部门禁
├─ 消费者使用本地 path/file 联调
├─ 修复所有消费者暴露的抽象缺陷
├─ 发布不可变 Foundation tag 与资产
├─ 消费者换成完整 Git rev / GitHub Release tgz
├─ 独立 checkout、无 sibling 仓库、完整产品门禁
├─ 产品断网运行编译后制品
└─ 更新 consumer-matrix 的真实状态与 commit 证据
```

路径依赖只用于发布前联调。永久 sibling `path`/`file:` 会让独立仓库 CI、GitHub checkout 和复现构建
依赖开发机目录；先发布再测试消费者则可能产生一个不可安装或语义错误的不可变版本。因此正确顺序必须
同时包含“发布前真实消费者联调”和“发布后不可变来源复核”。

## 3. 共享能力准入

### 3.1 提案必须具备的证据

| 问题 | 合格证据 | 不合格答案 |
|---|---|---|
| 为什么属于平台 | 责任边界、适用 Profile、参考实现与负责人 | “多个项目代码看起来相似” |
| 输入是什么 | 不可信/可信边界、编码、大小、是否可空、所有字段 | 一个理想化函数签名 |
| 失败怎样表达 | typed error、HTTP/CLI 映射、是否可重试、是否有副作用 | 返回 `anyhow::Error` 后各自猜 |
| 状态由谁拥有 | 数据库、Cookie、内存、文件、锁、发布树的明确所有者 | “Foundation 统一管理” |
| 最强约束是什么 | 所有消费者中的最严格规则和无法共享的产品规则 | 取最低共同实现 |
| 删除会怎样 | 具体破坏、替代实现、消费者迁移动作 | “应该没影响” |
| 如何验证 | 正例、边界、攻击/竞态/损坏负例、真实产品集成 | 只有 happy path 单测 |

### 3.2 准入结果

结论只有四种：由 Foundation 拥有；表达为通用 Profile/Capability；保留为产品 Adapter/业务语义；由
`sarmg-upgrade` 处理历史格式。单一参考消费者不是拒绝平台责任的理由，但能力必须完成规范、产品无关实现、
参考消费者迁移和旧实现删除这一整条纵向切片。Foundation 是构建期中央平台，不是中央运行服务。

## 4. 统一管理员认证流程

### 4.1 Server 登录流程

```text
POST /api/v2/auth/login
├─ HTTP adapter 收集全部 Origin field line
├─ 收集全部 Host field line，并加入 HTTP/2 URI authority
├─ 收集全部 Sec-Fetch-Site field line
├─ sarmg-admin-auth 严格同源校验
├─ sarmg-contracts 解析 exact {username,password}
├─ normalize_administrator_username
├─ validate_password（12..1024 bytes，无 ASCII control）
├─ Foundation 执行 body/IP/account/global 限流
├─ Foundation Store 加载当前管理员记录
├─ verify_password（只接受当前 Argon2id PHC）
├─ random_token 分别生成 Session/CSRF 所需随机值
├─ Foundation Store 只持久化摘要、固定 TTL 和 Session version
├─ Foundation HTTP Adapter 设置固定 Cookie
└─ 返回 exact AdministratorSession，role 固定 admin
```

`AdministratorLoginRequest` 的合同只允许一个 1–64 字节 printable ASCII username 候选通过 JSON 边界，
并不把 username 或密码认定为有效。username 规范化和密码策略必须在登录 admission 显式执行；持久化
凭据还应在启动时调用 `require_canonical_administrator_username` 与 `require_current_password_hash`，使不符合当前政策的数据库直接
启动失败。候选 guard 因此可以接受仍含 `@` 的 bounded printable 文本，但 canonical admission 必须拒绝；
这不是邮箱登录或旧字段兼容。JSON 字段只允许 `username`，`email` 是 unknown field。

### 4.2 Same-origin 流程

```text
框架保留所有原始 field line
├─ Origin：必须恰好一行、visible ASCII、无逗号
├─ Host/authority：合并后必须恰好一个权威值
├─ Sec-Fetch-Site：必须恰好一行且等于 same-origin
├─ ProductionHttps：Origin scheme 必须 https
├─ LoopbackDevelopmentHttp：scheme 必须 http，双方 host 都必须 loopback
├─ DNS lower-case / IP canonical / 默认端口规范化
└─ Origin authority 与有效 Host authority 精确相等
```

Foundation 没有 forwarded-header fallback。反向代理部署必须由产品在可信代理边界内先形成一个权威外部
Host，再把完整值交给 primitive；不能让库从冲突的 `Host`、`:authority`、`X-Forwarded-*` 中挑一个。

### 4.3 Mutation 与 CSRF

```text
浏览器持有当前内存 csrf_token
 -> unsafe method 由 admin-web/http-client 注入 X-CSRF-Token
 -> Server 收集全部同名 header line
 -> require_single_csrf_token 检查唯一且为 43 字符 canonical token
 -> require_csrf_token_matches_hash 常量时间比较持久化 SHA-256
 -> 产品再执行 Session、权限、业务 validation 和 transaction
```

safe method 不自动携带 CSRF。调用方不能直接设置 `X-CSRF-Token` 绕过 client 所有权；Server 不能只读取
第一个同名 header。Cookie 名称、属性、Session TTL、撤销、并发上限和安全审计由 Foundation Profile 固定，
产品不得覆盖。

### 4.4 角色流程

控制面身份只有 Administrator。数据库不需要 `role` 列；`AdministratorRole` 只有 `Admin` 一个枚举值，
wire 固定 `role:"admin"`。产品的数据面仍可有设备 credential、配对 token、摄像头凭据、媒体资源的
`role=primary/thumbnail` 等业务概念，但这些不是管理 RBAC，不得复用管理员角色字段。

## 5. 管理员 Web 流程

### 5.1 创建 client

```text
应用启动
├─ createAdministratorApiClient()
│  ├─ 浏览器默认 baseUrl = 当前 origin 根
│  ├─ 显式 baseUrl 仍必须与浏览器 origin 相同
│  ├─ Node 环境必须显式传绝对 HTTP(S) baseUrl
│  └─ Session/CSRF 保存在 closure，不进入持久存储
├─ useAdministratorSession(client)
│  ├─ 初始 loading
│  ├─ 自动 restore
│  ├─ 401 -> anonymous
│  ├─ 合同/网络错误 -> error
│  └─ 成功 -> authenticated
└─ 产品根据 phase 渲染登录页或业务页面
```

### 5.2 竞态流程

- login/logout mutation 进入同一 Promise tail，按调用顺序与 Set-Cookie 副作用一致完成。
- 每次 login、logout 或当前 401 都推进 generation；较早网络响应不得复活被替换的 Session。
- `transportSession` 只在闭包内追踪已发生的 cookie mutation，使紧随 login 排队的 logout 能携带正确
  CSRF；它不向 UI 暴露第二份授权状态。
- 并发 `restore()` 复用单一 Promise；较新的 login/logout 会使旧 restore 变为 superseded。
- React hook 自己也维护 generation 与 active client ref，组件切换 client 或卸载后，旧 Promise 不更新
  新组件状态。
- 业务请求发出时记录 Session/generation；旧 Session 的延迟 401 不得清除之后成功登录的新 Session。

### 5.3 产品响应验证

`AdministratorSession` 由共享 guard 验证。其他 API 返回值必须由产品传入 guard：

```ts
const value = await administratorApi.request(
  "/api/v2/example",
  isExampleResponse,
);
```

禁止传入永远返回 `true` 的 guard 或用 TypeScript 泛型冒充运行时验证。文件上传、下载、WHEP/HLS 等非 JSON
数据面可使用产品专用 transport，不强行走 `requestJson`。

## 6. 跨语言合同变更流程

```text
提出字段/格式变化
├─ 判断是否真的是新的唯一当前合同
├─ TypeScript type
├─ TypeScript runtime guard
├─ JSON Schema
├─ 正反 fixture
├─ Rust serde type + validate
├─ Rust 直接 include 同一 fixture
├─ 产品 router/client/离线工具适配
└─ 删除所有被替代字段、reader、fixture 和文档
```

### 6.1 必测差异

- 缺字段、未知字段、显式 `null`、空字符串不是同一状态；
- JSON integer 必须不超过 `Number.MAX_SAFE_INTEGER`，即使 Rust 可以解析更大 `u64`；
- source revision 必须是 40 位小写 hex，SHA-256 必须 64 位小写 hex；
- identifier 有长度和 ASCII 字符集，不接受展示文本；
- State 的 schema 字段必须出现，但可以为 `null`；
- Backup resources 至少一个且 `files >= 1`；
- 管理员 Session 的 `authenticated` 只能是 `true`，`role` 只能是 `admin`，username/token 必须 canonical。

### 6.2 产品加强规则

共享 guard 只验证跨产品 wire 最低合同。资源名称是否唯一、资源排序、path 是否 canonical、Schema 是否
等于产品 current identity、release target 是否等于 Server target，都必须由产品在共享验证成功后继续
检查。

## 7. Schema Identity 与 SQLite 流程

### 7.1 纯算法流程

```text
数据库 adapter 读取 sqlite_schema
├─ 排除 sqlite_* 和 product_metadata
├─ 取 type/name/tbl_name/sql 四列
├─ BINARY 顺序按 type/name/table 排序
├─ 映射 SchemaRow（拒绝 excluded/duplicate/out-of-order）
├─ 每字段：u64 big-endian UTF-8 byte length + 原始 bytes
└─ SHA-256 -> lowercase schema fingerprint
```

SQL 文本不做 formatter、空白归一化或语义等价转换；字节不同就是当前 Schema 不同。算法独立于 SQLx/
rusqlite，防止两个驱动同时链接 native SQLite，也使离线工具可以复用同一 fingerprint。

### 7.2 打开与校验流程

```text
产品完成路径/no-follow/owner/mode/实例锁/是否允许创建的判断
├─ 已有状态 -> open_existing（missing 是 typed failure）
└─ 明确初始化 -> create_if_missing
   └─ 每个连接：WAL + foreign_keys=ON + synchronous=FULL + busy_timeout=5s
      ├─ 产品执行当前 DDL 或进入当前运行
      ├─ 验证 product_metadata 五列 DDL 与 PRAGMA shape
      ├─ 验证唯一 singleton row 和 SQLite storage class
      ├─ 计算实际 fingerprint 并对比声明 hash
      └─ require_current_schema 对比 application/version/revision/hash
```

`integrity_check`、`foreign_key_check` 与 TRUNCATE checkpoint 是可组合诊断 primitive。checkpoint busy 或
incomplete 都是失败，不能在仍有 writer/reader 时只复制 main SQLite 文件。Foundation 不执行 migration、
backup、restore 或业务 transaction。

## 8. Server Target 流程

每个 Server binary crate 同时依赖 `sarmg-server-target` 并在其 build/release/start 层声明同一 target：

```text
cargo build --target x86_64-unknown-linux-gnu
├─ sarmg-server-target compile_error 拒绝其他 arch/OS/libc/pointer width
├─ 产品 build.rs 可在更早阶段给出产品名错误
├─ release identity target = x86_64-unknown-linux-gnu
├─ 归档检查 ELF machine = x86-64
└─ 启动脚本检查 Linux x86_64/glibc（若产品提供脚本）
```

Server target 统一不等于所有代码只能 AMD64。Host Monitor Client、Android/iOS 客户端、移动 FFI 和其他非
Server binary 不依赖该 crate，继续按自己的平台矩阵构建。Foundation 和 Upgrade 没有在线业务 Server；
Foundation 发布 identity 使用 `source-any`，Upgrade 的 CLI 发行目标由其自身合同约束。

## 9. Web 工具链与 package 流程

### 9.1 产品 Web

除 Dufs 外，每个产品 Web 位于 `clients/web` 并执行：

```text
package.json + .node-version
├─ assertAdministratorWebToolchain 检查精确 Node/React/Vite/TS 版本
├─ createSarmgReactViteConfig 建立 React plugin 与 clean dist
├─ TypeScript strict typecheck
├─ Vite build
├─ 产品业务 guard / UI / accessibility tests
└─ Server release 嵌入或携带 dist，并做完整性验证
```

消费者目前使用 npm 与 `package-lock.json`；Foundation monorepo 使用 pnpm 与 `pnpm-lock.yaml`。共享断言不
强制消费者改用 pnpm。Dufs 选择正式的 `web-embedded-native` Profile：原生 ES modules 被嵌入单
binary，同时采用统一 Admin Client、设计令牌和字体资产。这是通用运行形态，不是产品特判。

### 9.2 Foundation package

```text
修改 src / Schema / fixture / CSS
├─ clean 删除旧 dist，且拒绝 linked dist
├─ TypeScript --noEmit
├─ build 当前 JS/d.ts
├─ 复制公开 JSON/CSS/tsconfig 静态入口
├─ test 从 dist 导入
├─ 检查 manifest / exports / files / peers
├─ pnpm pack 生成 4 个真实 tgz
├─ 检查 tar canonical path、duplicate、link、special file、意外源码
└─ 临时空目录 npm --offline --ignore-scripts 安装并解析所有 export
```

`contracts` 是 `http-client` 的精确 peer；`admin-web` 精确依赖 contracts/http-client 并以可选 peer 声明
React/Vite 入口所需包。tarball 内不得保留 `workspace:`，也不得依赖 monorepo symlink 才能运行。

## 10. Design Token 变更流程

```text
确认属于平台设计语义
 -> 定义 primitive/semantic 名称与适用 Profile
 -> 更新 TypeScript token
 -> 更新 light CSS
 -> dark 只覆盖真正变化项
 -> 更新 scoped reset/accessibility（如适用）
 -> source/dist/effective value 一致性测试
 -> 所有消费者视觉、键盘、高对比度和 reduced-motion 验证
```

不得因为多个产品都使用“按钮”就把完整组件、品牌、页面 shell 或主题存储移入 Foundation。删除 token 时
直接删除并升级消费者，不留下重复 CSS custom property alias。

## 11. Consumer Matrix 流程

| 状态 | 含义 | 允许的证据 |
|---|---|---|
| `not-migrated` | 尚未进入目标 Profile 迁移 | 不得声称平台能力已通过 |
| `migration-in-progress` | 正在完成纵向切片 | 必须列出当前证据和未完成项 |
| `conforming` | 不可变来源、独立 checkout 和全部门禁通过 | 必须有完整 40 位验证 commit 且无例外 |
| `non-conforming` | 已声明 Profile 但当前验证失败 | 保留真实失败状态，不伪装绿色 |
| `temporary-exception` | 迁移期存在已登记例外 | 例外必须有期限且不得降低安全下限 |

`packages` 只列消费者直接采用的 Foundation 组件，不能把传递依赖或相似本地实现算作已采用。每次发布后
必须用最终消费者 commit 更新矩阵；矩阵 Schema 与 `check-foundation.py` 会拒绝未知仓库、未知组件、重复
项和自相矛盾状态。

## 12. Release 流程

```text
main 工作树完全干净
├─ 全部 Rust/Web/Python/package 门禁通过
├─ Cargo/npm/policy/consumer matrix 版本一致
├─ 十三个 Cargo package 均携带审核过的根 LICENSE
├─ 创建唯一 annotated v0.5.0 tag
├─ push tag 触发唯一 release job
├─ 再次运行全部门禁
├─ 生成 4 个 npm tgz
├─ 生成 deterministic sarmg-release-tool tar.gz
├─ 写 state-contract.json（Foundation 无 runtime state）
├─ SHA-256 绑定五字段 release-identity.json
├─ 写 toolchain/lock/component/asset build-inventory.json
├─ 写覆盖所有非自身 asset 的 SHA256SUMS
├─ 写并自验 link-free exact release-tree.json
└─ 创建 GitHub Release；已存在同名 release 时失败，不覆盖
```

release tree manifest 位于 `artifacts/` 之外且不描述自身，避免递归 hash。普通 workflow 的权限是空顶层加
job `contents: read`；仅 tag-only release job 可用 `contents: write`。action 必须锁完整 SHA，checkout
不得持久化 credential。

## 13. 删除与破坏性变更

### 13.1 删除清单

删除一个公开能力时同步处理：Cargo member/dependency、npm workspace/dependency/peer/export、源码、测试、
fixture、Schema、lockfile、package smoke、release inventory、consumer matrix、各产品调用和全部中文文档。
不得留下 deprecated symbol、alias package、旧 CSS property 或永远不再调用的 parser。

### 13.2 持久状态变化

```text
新实现需要改变持久状态
├─ 当前仍是开发期、无受支持发布状态
│  └─ 直接定义新当前 Schema；测试数据重建，不写兼容代码
└─ 已有明确受支持的稳定 source/target
   └─ 在线产品仍只读 target current；sarmg-upgrade 新建精确离线 edge
```

密码 policy 改变也遵循相同原则：在线 Server 不尝试多个参数集；启动/登录只接受当前 hash，转换必须在
独立受审流程完成。

## 14. 提交与最终交付

按“一个大问题一个提交”组织历史，例如：统一认证与合同、Web 基线、Server target、Schema/SQLite、发布
供应链、中文文档与消费者采用分别提交。提交前确认没有把 `target`、`node_modules`、`dist`、测试数据库、
真实 Secret 或本地 release 资产带入 Git。

最终交付必须同时满足：Foundation 全部门禁通过；每个消费者使用不可变依赖；各产品自己的测试和 release
验证通过；Server 非 AMD64 compile-fail；Dufs 例外被明确记录；管理面只有 admin；current-only 扫描无旧
名称/旧字段/双路径；Git 按大问题提交并推送；任何尚未完成的外部发布步骤被明确报告而不是假定成功。
