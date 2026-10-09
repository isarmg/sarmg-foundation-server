# 公共配置、诊断、锁与日志

本文描述0.11.0的公共实现。精确版本与不可变源码/资产分别记录，公开API只提供通用机制。

## 配置与入口

`xcss-config::resolve` 接受产品的 typed 默认配置、可选 JSON 文件、明确映射的环境覆盖和显式命令行覆盖。优先级为命令行、环境、文件、默认值；每一层在更高层覆盖之前按产品 Serde 结构校验。未知字段、重复字段、缺失字段、错误类型、结构数组及无效覆盖路径失败，不打印提交值。文件与环境 JSON 输入上限为 1 MiB，覆盖条目最多 256，数组只能整体替换。

`resolve_validated` 另接受 `Fn(&T, ConfigSource) -> Result<(), ConfigError>`，用于每层已有字段的内在约束，如地址语法、数值范围和密钥编码。尚未提供的必需凭据，以及路径、数据库、设备等外部前提，应在最终有效配置阶段验证；否则合法 CLI 覆盖无法修正默认部署路径。`Loaded.sources` 以 JSON pointer 返回每个有效叶字段的来源，不包含字段值。产品维护业务字段、默认值、环境映射及最终必要约束。

Unix `read_private_file` 只读一个 700 私有目录中的 600 文件，要求当前用户所有、普通文件、单硬链接、无符号链接和稳定身份，读取上限 1 MiB。它不创建目录或锁文件。初始化与安全写入由产品显式执行，并复用既有文件安全实现。

`xcss-server-cli` 提供机器错误输出、`CliError` 的安全 anyhow 边界和状态查询。机器错误 stdout 仅一个 `ErrorEnvelope` JSON 记录，失败退出码非零。`query_status` 在 3 秒和 4 KiB 边界内实际请求 `/readyz`，禁止代理和重定向，核对 `x-xcss-service` 及 HTTP 200/503 与 `ready` 的一致性。服务身份由产品外层 `service_identity_middleware` 注入。核心命令及业务成功条件由产品定义。

`ContractJson<T>`、`ContractQuery<T>`、`ContractPath<T>` 委托 Axum 的当前解析器，将拒绝转换为公共错误结构、安全 `details.reason` 与 `Cache-Control: no-store`。`MISSING_FIELD`、`UNKNOWN_FIELD`、`TYPE_MISMATCH`、`JSON_STRUCTURE`、语法、媒体类型、查询和路径错误分别表达，提交键名和值不进入错误响应；请求体超限保留 HTTP 413 和 `payload_too_large`。产品仍须使用当前 DTO 约束字段和业务语义；包装器不会自行补历史字段别名。

`request_context_middleware` 复用 native runtime 的同一请求边界：验证单个 bounded `x-request-id` 或生成编号，将 typed `RequestId` 和 String 放入 extensions 并写入响应头。HTTP 400/500 的未编码 JSON 错误结构在 16 KiB、1 秒内附上同一编号和 no-store；成功流不缓冲，非错误结构的小 JSON 保留原值，过大/停滞错误体转安全内部错误。该层应在产品鉴权和输入中间件之外。

`create_empty_private_directory` 仅接受空目录及合法公共锁文件，不覆盖未知状态。`create_runtime_log_directory` 只在显式初始化创建私有 `logs/`；`validate_runtime_log_directory` 只读验证既有目录。静态管理员只读检查复用 `StaticAdministratorStore::validate_persistent_accounts`，核验当前文件结构、账户及配置 ID，不保存、不创建账户或会话。

## 一致的写入权

`.xcss-maintenance-pending.json`的任何目录项都阻止正常运行和在线写探针，包括损坏文件或悬空链接。独占维护锁允许持有者处理该门；普通共享锁不绕过它。`release(self)`在业务资源全部关闭后显式unlock，异常退出由描述符关闭释放。

## 只读数据库校验

Linux `xcss-sqlite::open_validation_snapshot` 对源主库及现有 WAL/journal 建立稳定、有限的临时副本，读取 SHM 锁状态但不复制 SHM，不让 SQLite 打开或恢复源文件。复制及复读摘要、元数据和侧车存在性均需一致；活跃 writer 或不支持的原生锁环境可控失败。默认总量 4 GiB、10 秒、最多 3 次捕获尝试。SQLite 仅打开临时私有副本，允许副本恢复随后以 query-only pool 校验；产品仍验证实际 schema 和业务规则。pool 克隆保留临时目录 guard。

诊断捕获必须在**不持有源数据库 SQLx/SQLite 连接的独立诊断进程**执行。POSIX 记录锁以进程为单位：关闭同 inode 的任意原始描述符可能释放该进程 SQLite 锁。运行路径须先完成并关闭原始文件校验，再打开数据库；打开后采用元数据身份检查，不重新打开并关闭原始同 inode 文件。这是诊断临时副本，不是可恢复备份。

## 通用结构化日志

`xcss-log` 是不依赖 Server target/profile/runtime 的中立叶模块，可被 Server 与跨平台 Client 直接消费。`LogRecord` 使用 UTC RFC3339 毫秒时间、固定等级、service/component/event/message/scope、稳定实例 ID，以及可选 request/task/error 标识。公共事件来自 `CommonEvent` 的唯一等级和消息模板，产品事件必须使用传入服务命名空间。实例事件不能冒充 Server 初始化或生命周期事件。

秘密字段递归改为 configured 标志，带凭据 URL、普通文本中的凭据模式和控制字符经过统一处理。任意内部错误链不得作为普通业务 message 或自由属性传入。记录最多 16 KiB、消息最多 2 KiB；序列化在写入前完整检查，超限不产生半条记录。`query` 可精确筛选实例、scope、UTC 时间范围、最低等级、事件和 request/task ID，输入及结果有上限，坏记录明确失败。

`emit_stderr` 显式写入 stderr，保留由实际捕获它的外部管理器决定。Windows SCM 没有 stderr 持久化保证。`install_rotating_file(sink)` 只允许安装一次进程持久出口；`LogRecord::emit()` 和默认 tracing Layer 共同写入 typed sink，未安装时写 stderr。显式输出失败返回错误，tracing 失败计入 `rejected_count`，消费者必须处理。

Unix `RotatingLogFile` 使用既有 700 目录、600 单链接文件和一个 writer lock；默认单文件 8 MiB、4 个归档，共 40 MiB。`open(directory,stem,retention)` 使用 `stem.jsonl` 与 `stem.N.jsonl`；`open_file(active_path,retention)` 保留用户选择的确切文件名，归档为 `filename.N`。超出声明保留策略的既有归档、坏权限或链接会失败，启动不清理未知数据。

Windows `RotatingLogFile::create_private(directory,stem,retention)` 在已经过应用状态权限校验的物理父目录中，只创建最终日志目录。目录与文件 owner 必须是当前进程用户 SID，protected DACL 精确允许该 SID、SYSTEM、Administrators；文件普通、单链接且非 reparse point，目录树全程用不允许 delete sharing 的原生 handles 固定。`open` 仅验证既有对象，不修复 ACL。日志名为 `stem.jsonl`，归档为 `stem.jsonl.N`，同样默认 40 MiB；目录专用于这一 sink，不允许未知条目。创建和写入由服务自身身份执行，管理员查询可使用其 OS 授权打开文件并传入 `query`，不需要创建或改变日志 ACL。同一 OS 身份下的进程属于同一信任范围。

Windows writer 独占租约持续至 sink 关闭，写入前重新验证固定目录、writer 和 active file 的 ACL、类型、链接数及长度。任何存储错误都使该 sink 拒绝后续追加，产品需报告安全日志错误。每条成功写入先完成 bounded 序列化，再写入并 `sync_all`；坏记录不写，半条 I/O 失败不会被查询解释为成功。

可选 `tracing` feature 的 `FoundationStructuredLayer` 复用 typed records，继承 span 的结构化关联字段，隐藏 legacy 文本和内部错误链；未知普通事件用安全产品 diagnostic 模板，未知 common 事件拒绝。`with_writer` 与 `with_rotating_file` 构造 sink；`set_rotating_file(&self, sink)` 可在已安装 subscriber 中、完整运行前提校验后切换共享输出。`rejected_count` 包含字段/记录拒绝和 writer 错误，产品必须观测并表达日志降级或失败。

## 验证边界

公共测试覆盖严格配置分层、秘密不回显、真实flock互斥/显式释放/维护门、WAL 当前提交与源字节不变、HTTP 解析拒绝、精确日志筛选、实际文件轮转与 sink 身份变化。Windows 原生 job 必须通过受限令牌拒读写、ACL 不修复、junction/硬链接拒绝、实例精确筛选、容量和 typed/tracing 共用输出测试；Linux 交叉编译不替代它。状态查询另用临时 HTTP 监听验证服务身份和实际 readiness。这些公共测试不替代产品独立构建、实际发行物和设备验证。

## 行政维护权限桥（0.10.8）

普通 `PrivateStateDirectory::open` 与 `PrivateDirectory::open_existing` 仍要求 effective uid 精确等于目录所有者。离线维护可明确使用 `open_for_administration`：仅 effective uid 0 或实际目录所有者允许，逐级 NOFOLLOW 打开已有实体 0700 目录，不创建、不 chmod/chown 现有对象。新 common lock 和原子 pending 文件以排他新 descriptor 生成，再 fchown 为目录实际 uid/gid，保持 0600 和单链接；已有错误 owner、mode 或链接拒绝，绝不修复后继续。

普通`PrivateStateDirectory::open`和`PrivateDirectory::open_existing`要求effective uid等于owner。`open_for_administration`只接受uid0或实际owner，保留0700、NOFOLLOW和稳定身份；新锁/原子文件通过固定descriptor设为目录实际uid/gid、0600、单链接，已有不安全对象拒绝且不修复。`owner_uid/owner_gid`返回实际身份；`MaintenanceLock::as_fd`仅借用已持锁的描述符，不转移所有权或定义子进程操作协议。
