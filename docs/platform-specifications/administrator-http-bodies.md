# 管理员 HTTP 正文资源边界

所有权：Foundation Admin Core 的固定策略、Axum Adapter 的读取机制；Hyper 仅适配 Body 和真实 socket peer，
不实现第二套读取/限流逻辑。适用于持久管理员和静态管理员 Profile。设计输入来自 Dufs 的登录正文准入、
慢速流和取消释放测试；文件上传的流式协议、提交事务和容量控制仍由产品业务负责。

## 不变量

- 登录与管理员创建/改密只接受单个、完整解析的 `application/json` Content-Type，允许合法参数，拒绝重复字段和畸形参数。
- JSON 与无字段停用操作均最多读取 16 KiB，单次正文读取期限 10 秒。
- 一个已挂载的管理员 Router 及所有克隆共享全局 32 个、每真实来源 IP 4 个读取槽位；配置不得覆盖这些数值。
- IPv4-mapped IPv6 与对应 IPv4 共用来源桶；X-Forwarded-For 不参与登录来源判定或正文准入。
- 无排队读取；满额返回 429、`Retry-After: 1` 和可重试的 `auth.body_capacity`。来源 map 仅含活动读取者，最多 32 项。
- 已完成、超时、正文失败和调用方取消均释放槽位并删除空来源桶；锁中毒后拒绝新读取，不在未知计数上恢复。
- 超限返回 413；传输失败返回 400；超时返回 408。响应均为安全 ErrorEnvelope，保留经验证的 Request ID，不包含底层错误。
- 读取结束后才进入 DTO 校验与密码计算；正文槽位不占用 Argon2 槽位。管理员写入仍在事务提交时重新验证 Session/CSRF。

正文读取状态只存在内存，不改变持久格式，没有旧版 reader、升级 edge、兼容开关或回退路径。
不自动重放登录或写操作，HTTP 请求取消也不等于已经进入业务事务的操作被撤销。

## 验证与消费者迁移

Axum/Hyper 共用 Testkit：缺失/重复/畸形 Content-Type、四个悬挂正文、第五个请求伪造来源头仍被拒绝、取消后重新登录成功。
Adapter 单测覆盖全局上限、IPv4 映射、精确字节边界、虚拟时钟超时、取消资源释放及传输错误脱敏。

静态管理员配置同时限制 1024 个账户且拒绝重复 administrator_id；不同 username 不能共享会话主体。

登录在读取正文前拒绝重复 Cookie 字段行、重复当前 Session Cookie 以及无效当前 Token 形状，返回 `auth.invalid_cookie`，不设置新 Cookie。该规则由 Axum/Hyper 共享测试覆盖。登录失败预算按真实 socket IP（IPv4-mapped 地址规范化）和账户分别执行，不读取代理来源头；达到失败预算时返回 `auth.rate_limited` 和保守的完整平台窗口 `Retry-After`。
Dufs 的产品重复登录读取器、AccessControl/SessionStore 和 login_rate_limit 模块仍待在路由迁移时删除；
不能把上游测试通过视作 Dufs 接入完成。四个已迁移的持久 Server 自动通过当前 Adapter 获取上述保护，
仍需更新依赖锁并验证消费者；当前工作区实现未作为新不可变 Foundation 版本发布。
