# 03. Rust 管理员认证、错误、Server Target 与 SQLite

## 3.1 阅读顺序

建议按以下顺序阅读Rust crate，因为后者会复用前者：

```text
sarmg-admin-auth ───────────────┐
sarmg-error ────────────────────┼─> sarmg-contracts
sarmg-schema-identity ──────────┘
       └─> sarmg-sqlite

sarmg-server-target（独立，只给Server binary）
```

每读一个public函数都问四个问题：输入是否已经可信、失败是否typed、产品还需做什么、哪个测试证明负例。

## 3.2 管理员 Username

`normalize_administrator_username` 用于登录候选：先要求原始值为 1～64 个 printable ASCII bytes，
再 `trim_ascii` 并 ASCII lowercase，最后调用 canonical 验证。由于 control 已在第一步拒绝，这里的 trim
实际上只可能移除两端的 ASCII space（U+0020）。`require_canonical_administrator_username` 用于持久状态和
wire Session：它不修改输入，只接受已经规范的值。

```rust
use sarmg_admin_auth::{
    normalize_administrator_username,
    require_canonical_administrator_username,
};

let username = normalize_administrator_username(" Admin.Ops ")?;
assert_eq!(username, "admin.ops");
require_canonical_administrator_username(&username)?;
```

canonical username 为 3～64 bytes，首尾必须是小写 ASCII 字母或数字，全部字符只来自
`[a-z0-9._-]`。点、下划线和连字符只是本地标识符字符，相邻分隔符允许，没有域名语义。wire candidate
只负责在解析边界限制大小与字符可打印，因此候选中的 `@` 可以到达 admission，但必定无法成为当前身份。
`Admin` 只在登录候选规范化阶段变为 `admin`；持久 `Admin`、`admin@company.test`、`管理员`、首尾分隔符和
控制字符均拒绝。

产品启动时应遍历数据库中所有管理员并调用`require_canonical_*`。不要在启动时自动lowercase并回写，
否则Session、审计和外键可能被静默改变。

## 3.3 密码候选与当前Argon2id

plaintext policy：12～1024 UTF-8 bytes且无ASCII control。bytes和Unicode code point不是同一概念；wire
contract允许最多1024字符候选，Server仍会按bytes执行真正密码策略。

```rust
use sarmg_admin_auth::{hash_password, verify_password};

let phc = hash_password("correct horse battery staple")?;
assert!(verify_password("correct horse battery staple", &phc));
assert!(!verify_password("wrong-password", &phc));
```

当前PHC必须同时满足：Argon2id、v19、`m=19456,t=2,p=1`、16-byte salt、32-byte output，而且解析后重新
序列化必须与原字符串相同。一个使用不同参数但密码正确的Argon2id hash仍会返回false。

为什么严格到参数？因为“验证多种hash再登录后升级”会让在线产品永久携带多个安全政策。当前设计要求
启动时`require_current_password_hash` fail fast；确需转换时在独立受审流程重新建立当前hash。

## 3.4 Token与摘要

```rust
use sarmg_admin_auth::{random_token, token_hash, token_matches_hash};

let token = random_token()?;
assert_eq!(token.len(), 43);
let digest = token_hash(&token);
assert!(token_matches_hash(&token, &digest));
```

随机输入是32 bytes，编码为URL-safe Base64无padding。`is_token_shape`不仅看43字符和字符集，还decode并
重新encode，拒绝非canonical末尾bits。Session数据库通常只存SHA-256摘要；比较先验证当前shape和32-byte
expected digest，再使用constant-time equality。

随机源失败时必须让Session创建失败，不能回退时间戳、普通UUID或PRNG。CSRF token可与Session token分开
生成；具体持久字段、TTL和撤销属于产品。

## 3.5 Raw Cookie

`parse_cookie_value(cookie_header, name)`从一条raw Cookie header中提取唯一非空同名cookie。它拒绝非法
cookie name、空值和同一行重复名称。

框架adapter还必须保证raw Cookie header field line本身只有一条。如果框架先把多行合并再只传一个字符串，
Foundation无法知道原请求是否歧义。这是“library contract”和“framework integration”必须一起测试的例子。

## 3.6 Same-origin逐步解析

Server应把全部Origin值、全部Host值、HTTP/2 URI authority和全部Sec-Fetch-Site值复制为byte列表，再调用：

```rust
use sarmg_admin_auth::{
    AdministratorOriginMode,
    require_administrator_same_origin,
};

let verified = require_administrator_same_origin(
    AdministratorOriginMode::ProductionHttps,
    &[b"https://admin.example.test"],
    &[b"admin.example.test"],
    &[b"same-origin"],
)?;
assert_eq!(verified.port(), 443);
```

校验顺序：每种header必须恰好一行；visible ASCII且无逗号；Sec-Fetch-Site精确same-origin；Origin只有
scheme与authority；模式决定https/http；解析DNS/IPv4/bracketed IPv6和端口；HTTP开发模式双方必须真实
loopback；最后比较规范化authority。

生产behind proxy时，产品必须在可信代理边界形成一个外部有效Host。Foundation不读取X-Forwarded-Host，
也不会在Host与`:authority`冲突时挑一个。

## 3.7 CSRF

```rust
use sarmg_admin_auth::require_csrf_token_matches_hash;

require_csrf_token_matches_hash(&[csrf_header_bytes], &stored_digest)?;
```

helper要求X-CSRF-Token只有一行、无逗号、visible、canonical 43字符，再constant-time比较。产品调用顺序一般
为：同源检查→Session Cookie→Session TTL/version→CSRF→业务授权/validation→transaction。具体错误映射
和是否对登录端点要求同源由当前产品合同决定；当前Sarmg浏览器管理登录也要求完整同源header。

## 3.8 ErrorCode与RequestId

```rust
use sarmg_error::{ErrorCode, RequestId};

let code = ErrorCode::new("media.upload_conflict")?;
let request = RequestId::new("request:01J-test")?;
```

ErrorCode最长128 bytes，小写字母开头，只含小写字母、数字、`.`、`_`、`-`。RequestId为1～128 bytes，
允许ASCII字母数字和`._:-`。二者都适合日志/指标/dispatch，不适合展示给最终用户。

## 3.9 ErrorEnvelope

```rust
use sarmg_error::{ErrorEnvelope, HttpStatus};

let envelope = ErrorEnvelope::new(HttpStatus::TooManyRequests, "请稍后重试")
    .with_request_id("request-1")?
    .with_detail("retry_after_seconds", 5);
```

JSON固定包含`code`、`message`、`retryable`；`request_id`可缺失但不能显式null；`details`必须对象，空对象序列化
时省略。client按code/status分支，不解析message。内部SQL错误、raw上游body、密码和token不得放message/
details；产品日志自行脱敏。

HttpStatus提供常用400/401/403/404/409/422/429/500/503映射；默认只有429和503 retryable。产品仍应根据
操作幂等性决定是否真正重试。

## 3.10 Server Target

Server binary的Cargo依赖：

```toml
sarmg-server-target = {
  git = "https://github.com/isarmg/sarmg-foundation-server.git",
  rev = "<完整commit>",
  version = "=0.5.0"
}
```

crate root在非`x86_64-unknown-linux-gnu`直接`compile_error!`。产品release identity可通过
`SERVER_TARGET_TRIPLE`避免复制字符串，再用`require_server_target`检查外部metadata。

不要把依赖放到包含Server和客户端的workspace根公共crate里，否则Windows/macOS Agent也会compile-fail。
它应只存在于Server binary或Server专属library的依赖图。

## 3.11 Schema Identity基础

每个当前产品SQLite数据库有五列metadata表：singleton、application、application_version、schema_revision、
schema_sha256。完整身份必须四分量都匹配；revision数字只在某product/version内有意义。

Fingerprint读取除`sqlite_*`与`product_metadata`外的schema对象，按type/name/table BINARY排序。每个row的
type/name/table/sql分别加入8-byte big-endian UTF-8 byte length和原始bytes，再SHA-256。SQL不format、不
折叠空白：DDL字节变了就是Schema变了。

## 3.12 SQLx打开意图

```rust
use sarmg_sqlite::{open_existing, create_if_missing, PoolOptions};

let existing = open_existing(path, PoolOptions::new(8)).await?;
// 只有明确初始化流程才调用create_if_missing。
```

`open_existing`先检查path存在并且connect option不允许创建。`create_if_missing`明确允许SQLite建file，但它
不创建业务DDL。产品调用前仍负责no-follow、owner/mode、父目录、symlink、sidecar和实例锁。

每个连接固定WAL、foreign_keys=ON、synchronous=FULL、busy timeout 5秒；pool默认acquire timeout 10秒。
max必须>0，min<=max，acquire timeout非零。

## 3.13 数据库诊断与current校验

```text
read_schema_identity
├─ 查product_metadata DDL
├─ 查PRAGMA列shape
├─ 查每个值的SQLite storage class
├─ 要求唯一singleton row
├─ 算实际schema fingerprint
└─ 对比metadata声明hash

require_current_schema
└─ 在上述基础上再比expected product/version/revision/hash
```

`integrity_check`必须只返回一个`ok`；`foreign_key_check`收集具体table/row/parent/index；TRUNCATE checkpoint
的busy或frame不完整都返回错误。它们是诊断primitive，不自动修复数据库，也不替代backup维护锁。

## 3.14 常见误用

| 误用 | 为什么错 | 正确做法 |
|---|---|---|
| 对持久 username 每次登录 lowercase | 数据库身份本身不规范 | 启动 fail fast，管理流程建立 canonical 值 |
| 接受“更强参数”的Argon2 hash | current policy不唯一 | 只接受精确参数；变更发新current版本 |
| 只看token长度 | 可能非canonical编码 | 调`is_token_shape`/共享validator |
| 只读第一个Origin/Host | duplicate值有歧义 | 传全部field line并拒绝重复 |
| 用X-Forwarded-Host fallback | 未定义trusted proxy边界 | 产品先形成一个权威外部Host |
| `ErrorEnvelope.message`分支 | 文案不是machine合同 | 按code/status分支 |
| 所有workspace crate依赖target gate | 客户端被误限 | 只让Server依赖 |
| missing DB时改用create | 路径错误变空库 | 初始化与运行入口分开 |
| fingerprint前format SQL | 改变byte-exact合同 | 使用sqlite_schema原始sql |

## 3.15 本章练习

1. 列出一个合法与五个非法管理员 username，并标明哪条规则拒绝。
2. 解释为什么一个m=65536的Argon2id hash可能很强却仍非当前。
3. 构造Host与`:authority`冲突的请求，说明adapter应传几个host value。
4. 比较`open_existing`、`create_if_missing`和产品初始化DDL的责任。
5. 改变一个index SQL空格，预测fingerprint为何变化。
6. 在一个混合Server/Agent workspace中画出`sarmg-server-target`应放在哪个依赖节点。
