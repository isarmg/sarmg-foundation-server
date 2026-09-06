# Server Runtime 当前合同

生产服务通过 `ServerRuntime::builder` 注册产品身份、当前 Schema identity、健康检查、数值诊断探针和
后台任务，最后调用 `runtime.serve(HttpServer, product_service)`。这是唯一公开 HTTP 启动入口，不保留旧签名包装层。
`HttpServer` 持有 `BoundListeners`、`ProcessSignals`、`Http1Limits`、关闭预算及可选 `LifecycleParticipant`。
产品 Service 可以在 Axum Router 之前验证原始 URI；产品不再实现另一套信号监听或公共连接关闭循环。

## 生命周期

- Foundation revision 必须是完整的 40 位 Git commit；Schema 的产品和版本必须与 descriptor 一致。
- 健康和诊断探针并行读取，每项最多 2 秒；调用期间不持有诊断状态锁。panic/超时标记失败或 unavailable。
- 关键任务意外退出使服务 unhealthy 并启动关闭；Degrading 任务退出标记 degraded；BestEffort 不影响 readiness。
- 启动前的关闭请求不会丢失。正常关闭完成的任务标记 stopped；硬超时不执行 `abort_all()`，不假装未完成任务已经停止。
- 关闭阶段固定为 Quiescing、DrainingRequests、CancellingOrdinaryWork、DrainingCommits、ClosingState、Stopped。默认 30 秒正常排空，随后最多 10 秒强制通知普通工作退出。
- `ShutdownIncomplete` 返回非成功结果，携带未完成 socket、普通工作、提交数量及业务状态保留句柄。可执行程序负责明确非零退出，不能提前释放未完成工作仍依赖的锁和状态。已经运行的阻塞 I/O 不会因为等待者取消而消失。
- `WorkScope` 先关闭请求准入，等待请求和普通生产者，再关闭提交登记。`TrackedTasks` 的关闭与登记互斥，保留句柄不能在关闭后登记新任务。
- Request ID 只在公共 `request_service` 边界安装一次；`platform_router` 只组装路由，不重复安装全局 Body 限制。业务自己设置实际字节、并发和时间预算；流式文件不得经过整体收集。
- panic hook 不输出 payload、路径或请求内容。`panic=abort` 的正式二进制不具备任务 panic 隔离保证，进程异常退出后的状态恢复必须由产品验收。

## 连接预算

全部监听地址成功绑定后才初始化产品状态；空监听集合拒绝。动态端口在第一个地址确定后复用于其余零端口地址。
所有监听共享配额，先等待内核可读状态再取得许可，接受用户态 socket 前必须拥有许可。空闲监听不预占容量。
默认 HTTP/1 请求头期限 10 秒、缓冲 64 KiB、socket 写入空闲 30 秒。接受错误使用 50 ms 至 1 s 的有界退避，关闭可立即打断。
连接许可随实际 socket/升级后 I/O 对象存在，不随 Handler Future 返回而释放。Body 仍为流式。
Foundation 不认识产品路径、上传 ID、文件操作表或数据目录。`LifecycleParticipant` 只负责业务排空和最后关闭状态，不负责进程信号。

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

`PLATFORM_RESERVED_PATHS` 导出健康路径及管理员认证命名空间。文件服务必须只读预检冲突，保留实际路径/子树；不能无理由禁用整个 `/api` 目录。

`server-filesystem` 仅接受 Axum Adapter，保持原生内嵌 Web、静态管理员及内存 Session。该 Profile 的 `durable-operations` 也可由经验证的产品文件操作登记表实现，不要求把文件提交语义改成 Foundation 通用任务表。
