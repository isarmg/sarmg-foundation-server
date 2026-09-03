# ADR-0006：静态管理员 Profile

- 状态：Accepted
- 日期：2026-09-02

## 决策

`server-filesystem` 使用通用静态管理员 Store：配置提供当前 PHC 账户，Session 仅存内存并在重启后失效；
认证政策、限流、Cookie 和 Auth Router 仍由 Foundation 拥有。

## 后果

文件服务无需为管理员控制面引入持久数据库。这是任何同类嵌入式文件 Server 可选的 Profile，不是 Dufs
特判，也不允许产品保留自己的 AccessControl/SessionStore。
