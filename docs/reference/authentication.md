# 管理员认证原语

通常通过公共 HTTP 适配器使用这些原语。本文解释用户名、密码、令牌和请求头的具体规则；HTTP 正文预算见[适配器参考](../platform-specifications/administrator-http-bodies.md)。

## 管理员用户名

`normalize_administrator_username` 用于登录候选：先要求原始值为 1～64 个可打印 ASCII 字节，
再 `trim_ascii` 并 ASCII 小写，最后调用规范的验证。由于控制字符已在第一步拒绝，这里的 trim
实际上只可能移除两端的 ASCII space（U+0020）。`require_canonical_administrator_username` 用于持久状态和
通信格式会话：它不修改输入，只接受已经规范的值。

```rust
use xcss::admin_auth::{
    normalize_administrator_username,
    require_canonical_administrator_username,
};

let username = normalize_administrator_username(" Admin.Ops ")?;
assert_eq!(username, "admin.ops");
require_canonical_administrator_username(&username)?;
```

规范的 username 为 3～64 字节，首尾必须是小写 ASCII 字母或数字，全部字符只来自
`[a-z0-9._-]`。点、下划线和连字符只是本地标识符字符，相邻分隔符允许，没有域名语义。通信格式 candidate
只负责在解析边界限制大小与字符可打印，因此候选中的 `@` 可以到达准入，但必定无法成为当前身份。
`Admin` 只在登录候选规范化阶段变为 `admin`；持久 `Admin`、`admin@company.test`、`管理员`、首尾分隔符和
控制字符均拒绝。

产品启动时应遍历数据库中所有管理员并调用`require_canonical_*`。不要在启动时自动小写并回写，
否则会话、审计和外键可能被静默改变。

## 密码候选与当前Argon2id

密码明文规则：12～1024 UTF-8 字节且无ASCII 控制字符。字节和Unicode 码点不是同一概念；通信格式
contract允许最多1024字符候选，服务端仍会按字节执行真正密码策略。

```rust
use xcss::admin_auth::{hash_password, verify_password};

let phc = hash_password("correct horse battery staple")?;
assert!(verify_password("correct horse battery staple", &phc));
assert!(!verify_password("wrong-password", &phc));
```

当前PHC必须同时满足：Argon2id、v19、`m=19456,t=2,p=1`、16-byte salt、32-byte output，而且解析后重新
序列化必须与原字符串相同。一个使用不同参数但密码正确的Argon2id hash仍会返回false。

当前 `require_current_password_hash` 在启动时验证这组 PHC 参数；参数更新与产品的凭据处理一起规划。

## Token与摘要

```rust
use xcss::admin_auth::{random_token, token_hash, token_matches_hash};

let token = random_token()?;
assert_eq!(token.len(), 43);
let digest = token_hash(&token);
assert!(token_matches_hash(&token, &digest));
```

随机输入是32 字节，编码为URL 安全的 Base64无填充。`is_token_shape`不仅看43字符和字符集，还解码并
重新编码，拒绝非规范的末尾bits。会话数据库通常只存SHA-256摘要；比较先验证当前shape和32-byte
expected digest，再使用恒定时间 equality。

随机源失败时必须让会话创建失败，不能回退时间戳、普通UUID或PRNG。当前 `admin_core` 从同一会话令牌稳定派生 CSRF，不另生成随机 CSRF；会话表、TTL、撤销和 Cookie
政策由公共实现固定，产品负责初始化与挂载。

## 原始 Cookie

`parse_cookie_value(cookie_header, name)`从一条raw Cookie header中提取唯一非空同名cookie。它拒绝非法
cookie name、空值和同一行重复名称。

框架适配器还必须保证raw Cookie header field line本身只有一条。如果框架先把多行合并再只传一个字符串，
xcss无法知道原请求是否歧义。这是“library contract”和“framework integration”必须一起测试的例子。

## 同源校验逐步解析

服务端应把全部Origin值、全部Host值、HTTP/2 URI authority和全部Sec-Fetch-Site值复制为字节列表，再调用：

```rust
use xcss::admin_auth::{
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

校验顺序：每种header必须恰好一行；visible ASCII且无逗号；Sec-Fetch-Site精确同源；Origin只有
协议方案与authority；模式决定https/http；解析DNS/IPv4/bracketed IPv6和端口；HTTP开发模式双方必须真实
回环地址；最后比较规范化authority。

生产behind proxy时，产品必须在可信代理边界形成一个外部有效Host。xcss不读取X-Forwarded-Host，
也不会在Host与`:authority`冲突时挑一个。

## CSRF

```rust
use xcss::admin_auth::require_csrf_token_matches_hash;

require_csrf_token_matches_hash(&[csrf_header_bytes], &stored_digest)?;
```

辅助函数要求X-CSRF-Token只有一行、无逗号、visible、规范的 43字符，再恒定时间比较。产品调用顺序一般
为：同源检查→会话 Cookie→会话 TTL/version→CSRF→业务授权/validation→事务。具体错误映射
和是否对登录端点要求同源由当前产品合同决定；当前Xcss浏览器管理登录也要求完整同源header。

## WebSocket 来源

`require_administrator_websocket_origin` 校验完整 Origin/Host，Fetch Metadata 可缺失，存在时须为唯一 `same-origin`。普通 HTTP 管理请求继续要求完整同源字段；来源检查之后仍需认证和授权。
