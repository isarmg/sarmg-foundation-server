# ADR-0006：静态管理员运行形态

- 状态：已采纳
- 日期：2026-09-02

## 决策

`server-filesystem` 使用通用静态管理员 Store：配置提供当前 PHC 账户，会话仅存内存并在重启后失效；
认证政策、限流、Cookie 和 Auth Router 仍由 xcss 拥有。

## 后果

文件服务无需为管理员控制面引入持久数据库。这是任何同类嵌入式文件服务端可选的运行形态，不是 Xczs
特判，也不允许产品保留自己的 AccessControl/SessionStore。
