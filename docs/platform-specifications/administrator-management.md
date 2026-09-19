# 单管理员账户自助更新

Foundation 管理面只有一个活动管理员。唯一浏览器账户修改接口是
`POST /api/v2/platform/administrators/self`，由 Core、Store、Axum/Hyper 和 admin-web 共同实现。

```json
{"username":"admin","current_password":"current password","new_password":"new password"}
```

即使只改用户名，也必须验证当前密码。`new_password` 省略或空字符串表示保留密码。
成功为 204；账户 ID 不变，会话版本递增，该账户全部会话失效，并原子写入账户更新和会话撤销审计。
客户端用 `updateAccount()` 调用；更新后重新登录。

旧列表、创建、指定 ID 改密和停用接口已删除，按未知接口处理，没有兼容 handler。
`ADMINISTRATORS_PATH` 只是平台保留命名空间，不能交给文件服务 fallback。
空库初始化 `bootstrap_administrator()` 和本机维护 `change_administrator_password()` 仍保留。
初始化完成后调用 `validate_all_administrators()`，必须恰好有一个合法活动账户才能接收管理请求。
空库、多条记录、非活动账户或非法 PHC 都返回错误；校验不改变账户、会话或审计，不自动转换历史状态。

纯静态 Store 不注册自助更新；带受保护账户文件的 Static Store 支持更新并按既有原子持久化规则保存。
产品通过 `supports_account_updates()` 判断能力，不复制认证实现。

## 授权与事务

请求须具备当前 Session、同源 Origin、唯一 Host/Cookie/CSRF、application/json，正文最多 16 KiB。
密码验证和哈希使用既有有界 Argon2 槽位，在事务外计算。取得 SQLite BEGIN IMMEDIATE 写锁后重新取时，
事务内复核会话状态、CSRF 摘要、活动账户、会话版本及验证过的密码哈希快照。
任何授权变化或审计失败都不能留下部分账户更新。错误当前密码返回 `admin.current_password_invalid`。

CSRF 在同一会话生命周期内稳定，恢复不轮换；touch 单调推进活动时间，不改摘要。
从旧随机 CSRF 语义切换后，旧会话须重新登录；不改数据库 DDL，不自动重写旧会话。

验证由共享 Adapter 协议、Store 原子性与只读校验、受保护账户文件重启测试及 Web 自助账户流程分层覆盖。
