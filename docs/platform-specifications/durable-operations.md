# Durable Operations 当前接口

业务只提供目标、请求/结果、执行器，以及确定失败是否可重试的分类。Foundation 拥有状态、领取、attempt、
租约、幂等和审计 outbox。Unknown 绝不自动重放；同一 namespace/target 的 Running 或 Unknown 阻止后续领取。

## 事务与 fencing

- `enqueue_in`：产品 desired state 和持久意图同事务。
- `claim_next_in`：产品 leader/admission 检查和操作领取同事务；独立 `claim_next` 使用 BEGIN IMMEDIATE。
- `apply_transition_owned_in`：业务结果、当前未过期 owner 的状态转换、outbox 同事务。
  产品必须在任何错误时放弃整个业务事务；平台接口的 savepoint 还保证调用者误提交时不会留下半个状态转换。
- `abandon_claim`：只能把捕获到的 owner/attempt/expiry 对应 Running 标成 Unknown，允许该租约已过期；
  不允许发布成功、恢复 Pending 或影响新一次领取。远端可能成功但本地提交失败时使用此接口。
- `recover_expired`：每批最多 128 个过期 Running，按捕获的领取信息标为 Unknown，不重新执行。
- `recover_running`：仅供持有独占实例锁的启动恢复；不能用于正常多执行器运行时的全量回收。
- `resolve_in`：明确人工确认成功/失败得到 Resolved；无法确认只允许 Unknown → DeadLetter。
  产品操作者审计与人工处理同事务，不提供“重试 Unknown”接口。
- `mark_audit_delivered_in`：产品以 event ID 幂等物化审计，和 outbox 确认同事务。确认失败时审计插入也回滚。

独立转换提交接口返回事务内读取的快照，不在提交后重新读取可能已经变化的状态。
所有产品故障分类应使用固定安全 code，不写入远端响应、凭据、URL 或内部异常详情。

回归覆盖事务回滚、审计插入故障、同目标串行、幂等冲突、owner/expiry/attempt fencing、过期回收、
人工确认和 no-replay。只支持唯一当前 Schema，不读取历史格式、不执行旧版升级。
