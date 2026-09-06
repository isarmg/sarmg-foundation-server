# Server Runtime 当前合同

生产服务通过 `ServerRuntime::builder` 注册产品身份、当前 Schema identity、健康检查、数值诊断探针和
后台任务，最后调用 `runtime.serve(listener, product_router)`。产品不再实现另一套信号监听或 HTTP 关闭循环。

## 生命周期

- Foundation revision 必须是完整的 40 位 Git commit；Schema 的产品和版本必须与 descriptor 一致。
- 健康和诊断探针并行读取，每项最多 2 秒；调用期间不持有诊断状态锁。panic/超时标记失败或 unavailable。
- 关键任务意外退出使服务 unhealthy 并启动关闭；Degrading 任务退出标记 degraded；BestEffort 不影响 readiness。
- 启动前的关闭请求不会丢失。正常关闭完成的任务标记 stopped；超时仍不退出的任务取消并标记 aborted。
- 后台任务 drain 最多 30 秒，随后 HTTP drain 最多 30 秒。关键任务失败向进程返回错误，而非成功退出。
- 所有生产路由使用统一 Request ID 校验/传递、默认请求体预算和 handler panic 错误响应。
  产品可为业务路由声明明确预算。panic hook 不输出 payload、路径或请求内容。

## HTTP 所有权

| 路径 | 身份要求 | 返回 |
|---|---|---|
| `/healthz` | 无 | 存活 204、不健康 503；空响应体 |
| `/readyz` | 无 | 200 或 503；仅 `{"ready":bool}` |
| `/api/v2/auth/*` | 由统一 Auth Adapter 决定 | 唯一当前管理员 wire 合同 |

诊断 HTTP 路由及处理器已移除，`/api/v2/platform/diagnostics` 对匿名和已登录请求均为 404。
运行时内部快照仅供任务监督与测试，不作为管理 Web 功能或 HTTP 数据接口。
Schema identity 是经产品启动校验的编译期当前身份；数据库实时状态由数据库健康探针给出。

数值 metrics 的字段固定为 audit_backlog、operation_backlog、spool_pending_bytes、spool_pending_records。
未接入的能力或失败/超时的读取返回 null，不把“无法得知”当成零，也不保留过期的成功值。
Operations Store 提供只读取计数的共享探针：audit 为尚未物化的 outbox 数，operation 为 pending/running/unknown 数。
探针不得读取或返回产品 payload、Secret 或文件路径。

Axum 和 Hyper 的管理员协议使用 `sarmg-testkit::assert_administrator_http_contract` 运行同一套验收断言，
覆盖重复 field line、Cookie 歧义、Origin/Host/authority、CSRF、body 预算、严格错误合同和 Request ID 传递。

不提供 `/health`、`/health/live`、`/health/ready` 或旧认证路由别名。
