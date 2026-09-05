# 持久管理员管理

所有权：Foundation `admin-core`、`admin-sqlite`、HTTP Adapter、contracts、admin-web 和 admin-shell。
适用 `server-control-plane` / `admin-persistent`；静态管理员 Profile 不注册管理路由，配置修改不属于 Web API。
本切片从 Sentinel 的系统管理员页面提炼；摄像头、录像、事件与业务审计仍由产品负责。

## 唯一当前接口

| 方法和路径 | 输入 | 成功响应 |
|---|---|---|
| GET `/api/v2/platform/administrators?limit=50&offset=0` | limit 1–100；offset 非负；拒绝未知和重复参数 | 管理员摘要数组 |
| POST `/api/v2/platform/administrators` | 严格 JSON `{username,password}` | 204 空正文 |
| POST `/api/v2/platform/administrators/{id}/password` | 严格 JSON `{password}` | 204 空正文 |
| POST `/api/v2/platform/administrators/{id}/disable` | 空正文 | 204 空正文 |

无角色分级、删除、改名或重新启用接口。所有账户都是管理员。ID 是平台不透明标识，不要求 UUID。
摘要仅含 administrator_id、username、active、created_at_micros、updated_at_micros、last_login_at_micros。
时间为非负安全 JSON 微秒整数，last_login 必须存在，可为 null；不输出密码摘要、Session version 或 Token。
Rust、TypeScript、JSON Schema 共用 fixture。所有响应禁止缓存；错误是脱敏 ErrorEnvelope 和验证后的 Request ID。

## 信任和资源边界

读请求要求当前管理员 Session。写请求另要求相同 Origin、单值 Host/Origin/Cookie/CSRF 及当前 CSRF 摘要。
JSON 写请求要求单个完整解析的 application/json Content-Type；正文上限 16 KiB，拒绝未知及重复 JSON 字段。
登录与管理写入共享 10 秒读取期限、全局 32/每真实来源 4 个槽位，详见 `administrator-http-bodies.md`。
密码哈希使用已有固定 Argon2 策略与共享有界计算槽位；输入不实现 Debug。容量失败返回可重试 503。
列表每页最多 100 条；Web 每页 50 条，取消离开的读取，不把密码或 Session 写入浏览器存储。

## 并发、崩溃及失败语义

密码哈希完成后取得 SQLite BEGIN IMMEDIATE 写锁，再读取实际时钟，不能用哈希前/锁等待前的时间放行。
在同一事务内重新检查会话存在、未撤销、idle/absolute 未过期、管理员 active、Session version 匹配、
CSRF 摘要仍匹配授权快照。其他请求撤销 Session、停用账户或恢复 Session 轮换 CSRF 后，旧快照不能写入。
SQLite 和 Static Store 的 CSRF 轮换/活跃时间刷新都比较读取时的摘要，并拒绝时间倒退；迟到的 touch 或
restore 不能把新的摘要覆盖回旧值。比较失败映射为失效 Session，存储故障仍单独映射为内部错误。

创建、密码修改、停用与对应安全审计必须原子提交；密码修改/停用同时增加 Session version、撤销该账户全部
Session 并写入 sessions-revoked 审计。审计 actor 来自重新验证的会话，subject 为账户名摘要；详情不含凭据。
停用检查最后一个活动管理员，与停用写入在同一写事务内；并发相互停用最多一个成功。
不保留绕过此检查的低层停用入口。CLI 当前密码维护与空库初始化是明确的运维入口，不是旧版兼容路径。

失效授权为 401；不存在为 404；重复账户、已停用状态和最后管理员保护为 409；输入为 400；存储失败为
脱敏 500。审计失败必须回滚账户、密码、版本和撤销写入。Web 不自动重放写入；提交响应丢失时先重新读取确认。
本切片只使用现有 Foundation 当前表，不改变 DDL；不实现历史升级、读取器或旧 `/users` 别名。

## 产品迁移与验收

产品通过 Foundation router 挂载接口、通过 `AdministratorsPanel` 展示管理页，不注册第二套用户 API。
共享危险确认对话框默认焦点在取消按钮；错误置于对话框内；失败密码清空；修改自己密码后重新恢复 Session，
收到 401 切回登录。静态账户产品不挂载该面板。

证据：SQLite 并发停用、审计故障回滚、密码撤销、CSRF 轮换与提交时钟测试；Axum/Hyper 共享 testkit；
Chromium/Firefox 创建失败/成功、停用失败/成功、自身改密、危险确认焦点和移动明暗主题 WCAG AA 验收。
当前工作区实现尚未作为新的不可变 Foundation 版本发布，消费者联调来源仍需在 P13 统一替换并独立检出验证。
