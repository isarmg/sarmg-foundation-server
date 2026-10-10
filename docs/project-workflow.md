# xcss 工作流程与流程树

## 1. 文档目标

本文描述一项能力从产品本地需求进入 xcss、形成可发布合同、被真实产品采用并最终受运维约束的
完整流程。xcss 是所有产品的上游平台规范和安全下限；一次看似很小的辅助函数修改可能同时改变服务端、Web、
SQLite、离线工具和发布树，所以“源码编译通过”只是中间节点，不是完成定义。

## 2. 全局流程树

```text
提出能力
├─ 是否属于平台责任？
│  └─ 是：xcss 定义唯一规范和实现
├─ 是否属于已知运行形态差异？
│  └─ 是：选择或新增产品无关 Profile/Capability
├─ 是否属于产品业务语义？
│  └─ 是：产品通过 Adapter/Trait 实现
└─ 是否属于历史持久格式？
   └─ 是：按当前状态合同处理，不要求历史转换

确定所有权
├─ 收集实际调用点、最强安全约束和删除后果
├─ 写唯一当前 API、输入上限、错误、并发和所有权
├─ 先同步实现、正例/负例、Schema/fixture、中文文档
├─ 运行 xcss 全部门禁
├─ 消费者使用本地 path/file 联调
├─ 修复所有消费者暴露的抽象缺陷
├─ 发布不可变 xcss tag 与资产
├─ 消费者换成完整 Git rev / GitHub Release tgz
├─ 独立 checkout、无 sibling 仓库、完整产品门禁
├─ 产品断网运行编译后制品
└─ 在产品仓库记录本次源码、CI 与正式发行物的实际验收结果
```

路径依赖只用于发布前联调。永久同级 `path`/`file:` 会让独立仓库 CI、GitHub 源码检出和复现构建
依赖开发机目录；先发布再测试消费者则可能产生一个不可安装或语义错误的不可变版本。因此正确顺序必须
同时包含“发布前真实消费者联调”和“发布后不可变来源复核”。

### 3.1 提案必须具备的证据

| 问题 | 合格证据 | 不合格答案 |
|---|---|---|
| 为什么属于平台 | 责任边界、适用运行形态、参考实现与负责人 | “多个项目代码看起来相似” |
| 输入是什么 | 不可信/可信边界、编码、大小、是否可空、所有字段 | 一个理想化函数签名 |
| 失败怎样表达 | 具有类型约束的 error、HTTP/CLI 映射、是否可重试、是否有副作用 | 返回 `anyhow::Error` 后各自猜 |
| 状态由谁拥有 | 数据库、Cookie、内存、文件、锁、发布树的明确所有者 | “xcss 统一管理” |
| 最强约束是什么 | 所有消费者中的最严格规则和无法共享的产品规则 | 取最低共同实现 |
| 删除会怎样 | 具体破坏、替代实现、消费者迁移动作 | “应该没影响” |
| 如何验证 | 正例、边界、攻击/竞态/损坏负例、真实产品集成 | 只有 happy path 单测 |

### 4.1 服务端登录流程

```text
POST /api/v1/auth/login
├─ HTTP adapter 收集全部 Origin field line
├─ 收集全部 Host field line，并加入 HTTP/2 URI authority
├─ 收集全部 Sec-Fetch-Site field line
├─ xcss::admin_auth 严格同源校验
├─ xcss::contracts 解析 exact {username,password}
├─ normalize_administrator_username
├─ validate_password（12..1024 bytes，无 ASCII control）
├─ xcss 执行 body/IP/account/global 限流
├─ xcss Store 加载当前管理员记录
├─ verify_password（只接受当前 Argon2id PHC）
├─ random_token 生成 Session，derive_csrf_token 从同会话令牌稳定派生 CSRF
├─ xcss Store 只持久化摘要、固定 TTL 和 Session version
├─ xcss HTTP Adapter 设置固定 Cookie
└─ 返回 exact AdministratorSession，role 固定 admin
```

`AdministratorLoginRequest` 的合同只允许一个 1–64 字节可打印 ASCII username 候选通过 JSON 边界，
并不把 username 或密码认定为有效。username 规范化和密码策略必须在登录准入显式执行；持久化
凭据还应在启动时调用 `require_canonical_administrator_username` 与 `require_current_password_hash`，使不符合当前政策的数据库直接
启动失败。候选运行时校验因此可以接受仍含 `@` 的 bounded printable 文本，但规范的准入必须拒绝；
这不是邮箱登录或旧字段兼容。JSON 字段只允许 `username`，`email` 是未知 field。

### 4.2 同源校验流程

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

xcss 没有 forwarded-header 回退。反向代理部署必须由产品在可信代理边界内先形成一个权威外部
Host，再把完整值交给原语；不能让库从冲突的 `Host`、`:authority`、`X-Forwarded-*` 中挑一个。

### 4.3 修改操作与 CSRF

```text
浏览器持有当前内存 csrf_token
 -> unsafe method 由 admin-web/http-client 注入 X-CSRF-Token
 -> Server 收集全部同名 header line
 -> require_single_csrf_token 检查唯一且为 43 字符 canonical token
 -> require_csrf_token_matches_hash 常量时间比较持久化 SHA-256
 -> 产品再执行 Session、权限、业务 validation 和 transaction
```

safe method 不自动携带 CSRF。调用方不能直接设置 `X-CSRF-Token` 绕过 client 所有权；服务端不能只读取
第一个同名 header。Cookie 名称、属性、会话 TTL、撤销、并发上限和安全审计由 xcss 运行形态固定，
产品不得覆盖。

### 4.4 角色流程

控制面身份只有 Administrator。数据库不需要 `role` 列；`AdministratorRole` 只有 `Admin` 一个枚举值，
通信格式固定 `role:"admin"`。产品的数据面仍可有设备凭据、配对令牌、摄像头凭据、媒体资源的
`role=primary/thumbnail` 等业务概念，但这些不是管理 RBAC，不得复用管理员角色字段。

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

- login/logout 修改操作进入同一 Promise tail，按调用顺序与 Set-Cookie 副作用一致完成。
- 每次登录、退出或当前 401 都推进 generation；较早网络响应不得复活被替换的会话。
- `transportSession` 只在闭包内追踪已发生的 cookie 修改操作，使紧随登录排队的退出能携带正确
  CSRF；它不向 UI 暴露第二份授权状态。
- 并发 `restore()` 复用单一 Promise；较新的 login/logout 会使旧恢复会话变为 superseded。
- React 钩子自己也维护 generation 与 active client ref，组件切换 client 或卸载后，旧 Promise 不更新
  新组件状态。
- 业务请求发出时记录 Session/generation；旧会话的延迟 401 不得清除之后成功登录的新会话。

### 5.3 产品响应验证

`AdministratorSession` 由共享运行时校验验证。其他 API 返回值必须由产品传入运行时校验：

```ts
const value = await administratorApi.request(
  "/api/v1/example",
  isExampleResponse,
);
```

禁止传入永远返回 `true` 的运行时校验或用 TypeScript 泛型冒充运行时验证。文件上传、下载、WHEP/HLS 等非 JSON
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
- 源码修订号必须是 40 位小写 hex，SHA-256 必须 64 位小写 hex；
- identifier 有长度和 ASCII 字符集，不接受展示文本；
- State 的 schema 字段必须出现，但可以为 `null`；
- Backup resources 至少一个且 `files >= 1`；
- 管理员会话的 `authenticated` 只能是 `true`，`role` 只能是 `admin`，username/token 必须规范的。

### 6.2 产品加强规则

共享运行时校验只验证跨产品通信格式最低合同。资源名称是否唯一、资源排序、path 是否规范的、Schema 是否
等于产品当前身份、发行 target 是否等于服务端编译目标，都必须由产品在共享验证成功后继续
检查。

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
rusqlite，防止两个驱动同时链接 native SQLite，也使离线工具可以复用同一结构指纹。

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

`integrity_check`、`foreign_key_check` 与 TRUNCATE checkpoint 是可组合诊断原语。checkpoint busy 或
incomplete 都是失败，不能在仍有 writer/reader 时只复制 main SQLite 文件。xcss 不执行 migration、
业务备份、业务状态恢复或业务事务；管理员会话恢复由 `admin_core` 的认证机制提供。

## 8. 服务端编译目标流程

每个服务端 binary crate 声明单个 `xcss` 依赖，按需使用 `xcss::server_target` 常量，并在 build/release/start 层声明同一 target：

```text
cargo build --target x86_64-unknown-linux-gnu
├─ xcss 根 build.rs 与 src/lib.rs 硬门禁拒绝其他 arch/OS/libc/pointer width
├─ 产品 build.rs 可在更早阶段给出产品名错误
├─ release identity target = x86_64-unknown-linux-gnu
├─ 归档检查 ELF machine = x86-64
└─ 启动脚本检查 Linux x86_64/glibc（若产品提供脚本）
```

### 9.1 产品 Web

所有当前 React 管理 Web（包括 Xczs）都执行以下流程；具体前端根目录由产品构建声明给出：

```text
package.json + .node-version
├─ assertXcssWebToolchain 检查精确 Node/React/Vite/TS 版本
├─ createXcssReactViteConfig 建立 React plugin 与 clean dist
├─ TypeScript strict typecheck
├─ Vite build
├─ 产品业务 guard / UI / accessibility tests
├─ xcss-build-server 编排 Web → Rust，构建规范目标
└─ 对实际 Server 的 web-assets 清单验收，发行只携带清单，Web 内嵌于二进制
```

消费者目前使用 npm 与 `package-lock.json`；xcss 单包使用 pnpm 与 `pnpm-lock.yaml`。共享断言不
强制消费者改用 pnpm。Xczs 当前选择 `web-react-admin` 运行形态：React/xcss 拥有登录、导航和页面
骨架，原生 ES modules 的文件业务控制器保留独占 DOM 区域；两者一同嵌入单 binary。这是明确的组件
所有权边界，不是第二套认证或前端入口。

### 9.2 xcss 软件包

```text
修改 src / Schema / fixture / CSS
├─ clean 删除旧 dist，且拒绝 linked dist
├─ TypeScript --noEmit
├─ build 当前 JS/d.ts
├─ 复制公开 JSON/CSS/tsconfig 静态入口
├─ test 从 dist 导入
├─ 检查 manifest / exports / files / peers
├─ pnpm pack 生成 1 个真实 xcss-web-1.0.0.tgz
├─ 检查 tar canonical path、duplicate、link、special file、意外源码
└─ 临时空目录 npm --ignore-scripts 安装并解析所有 export
```

`contracts`、`http-client`、`admin-web` 是同一个 `@xcss/web` 包中的内部模块，按公开子路径使用；
包只为外部 React/Vite 入口声明精确的可选 peer。发行归档内不得保留 `workspace:`，也不得依赖 monorepo 符号链接才能运行。

## 10. 设计令牌变更流程

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

`admin-ui`、`admin-shell`、设计令牌与字体模块已经提供公共组件、管理页面外壳和样式机制；产品专有的
品牌、业务页面及业务状态仍留在产品。新增公共 UI 能力须按职责和通用性评审，不能仅因外观相似而上移。删除令牌时
直接删除并升级消费者，不留下重复 CSS custom property 别名。

## 11. 产品接入验收

产品在自己的清单和锁文件声明实际采用的公共依赖、完整源码修订号、运行形态与能力。
发布后以同一源码 commit 进行独立源码检出、完整产品门禁和实际运行验收，核对正式资产的身份、
平台、摘要以及嵌入资源。产品的 CI 与发行保存这次结果；失败、未执行及待验证项如实记录。

公共实现自己的构建和发布由上游质量门负责，产品业务行为由对应产品的测试负责。上游检查通过
不能替代产品验收，旧记录也不能作为当前发布结果。

## 12. 发行流程

```text
main 工作树完全干净
├─ 全部 Rust/Web/Python/package 门禁通过
├─ Cargo/npm/policy 版本一致
├─ 唯一 Cargo package xcss 与 npm 包均携带审核过的根 LICENSE
├─ 创建唯一 annotated v1.0.0 tag
├─ push tag 触发唯一 release job
├─ 再次运行全部门禁
├─ 生成 1 个 npm tgz
├─ 生成 deterministic xcss-release-tool tar.gz
├─ 写 state-contract.json（xcss 无 runtime state）
├─ SHA-256 绑定五字段 release-identity.json
├─ 写 toolchain/lock/component/asset build-inventory.json
├─ 写覆盖所有非自身 asset 的 SHA256SUMS
├─ 写并自验 link-free exact release-tree.json
└─ 创建 GitHub Release；已存在同名 release 时失败，不覆盖
```

发行目录清单位于 `artifacts/` 之外且不描述自身，避免递归 hash。普通 workflow 的权限是空顶层加
job `contents: read`；仅 tag-only 发行 job 可用 `contents: write`。action 必须锁完整 SHA，源码检出
不得持久化凭据。

### 13.1 删除清单

删除一个公开能力时同步处理：唯一 crate 的公开模块与依赖、唯一 npm 包的 dependency/peer/export、源码、测试、
测试夹具、Schema、锁文件、软件包冒烟验证、发行文件清单、各产品调用和全部中文文档。
不得留下已弃用的 symbol、别名软件包、旧 CSS property 或永远不再调用的解析器。

### 13.2 持久状态变化

```text
新实现需要改变持久状态
├─ 当前仍是开发期、无受支持发布状态
│  └─ 直接定义新当前 Schema；测试数据重建，不写兼容代码
└─ 已有明确受支持的稳定 source/target
   └─ 在线产品仍只读 target current；不要求新增历史离线转换边
```

密码规则改变也遵循相同原则：在线服务端不尝试多个参数集；启动/登录只接受当前 hash，转换必须在
独立受审流程完成。

## 14. 提交与最终交付

按“一个大问题一个提交”组织历史，例如：统一认证与合同、Web 基线、服务端编译目标、Schema/SQLite、发布
供应链、中文文档与消费者采用分别提交。提交前确认没有把 `target`、`node_modules`、`dist`、测试数据库、
真实 Secret 或本地发行资产带入 Git。

最终交付必须同时满足：xcss 全部门禁通过；每个消费者使用不可变依赖；各产品自己的测试和发行
验证通过；服务端非 AMD64 编译失败；Xczs 的 React 外壳与原生业务模块边界明确；管理面只有 admin；
当前 API、字段和实现不保留旧版本专用回退；Git 按大问题提交并推送；任何尚未完成的外部发布步骤被明确报告而不是假定成功。
