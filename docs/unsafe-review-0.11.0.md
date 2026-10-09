> 项目名称已规范化，版本、提交、摘要及验收状态保持历史记录，不作为当前验收证据。未经规范化的原始文本仅保存在本次工作区审计备份中；本文件不是逐字原始记录。

# xcss 0.11.0 unsafe 审查

审查范围为本仓库 owned Rust 源码及 Windows 原生测试，目标工具链 Rust 1.99.0、SQLx 0.9.0、唯一 native SQLite 0.37.0、windows-sys 0.61.2。源码完整 revision、测试日志和实际 CI run 随发行收据记录；本文件随同该 revision 封存，不以词频代替逐函数判断。第三方驱动内部 unsafe 不属于本仓库 owned 源码，依赖来源和版本由 Cargo.lock 固定。

| 位置 / 函数 | 必要性与安全前置条件 | 验证及未证明边界 |
|---|---|---|
| `xcss::sqlite/native_limits::native_limit` | SQLx 暂无 safe `sqlite3_limit` API。仅在 SQLx `LockedSqliteHandle` 暂停 worker 并独占有效连接时调用同一 native 库；固定四种类别，参数先校验为正有界 i32，`-1` 仅查询，设置后逐个读回。指针、借用和回调均不逃逸。保留这一窄 FFI，不自制驱动。 | 真实巨大 BLOB 读写、SQL 长度、bind 数量及其他连接不受影响；无效参数在任何修改前拒绝。不是 SQLite 总堆、排序工作区或打开 schema 的内存上限。 |
| `xcss::sqlite/native_security::set_and_verify` | SQLx 无 safe db-config API，SQLite 无等价 defensive PRAGMA。独占 SQLx handle / 暂停 worker、同一 pinned native library；固定 SQLITE_DBCONFIG_DEFENSIVE 的 C int 1/-1 和活 int 输出指针，设置后读回1，指针不逃逸。保留这一必要窄 FFI。 | 真实 writable_schema UPDATE 被拒、指纹字节保持，正常 INSERT/SELECT 可用；另一独立未加固连接同查询真实成功，证明不是坏 SQL 假通过。 |
| `xcss::sqlite/validation_snapshot::lock_read` | Linux `fcntl(F_OFD_SETLK)` 的 SQLite byte-range read lock；活 File 保持 FD，完整初始化 flock / zero PID。普通 POSIX lock 或整文件 flock 不具有所需相同语义，不能安全机械替换。 | WAL-only 当前代、busy、别名、输入 identity / bytes 不变、超限与坏库拒绝。实际 SQLx 打开前释放同 inode 原始 FD；副本内只读，源不打开 SQLite pool。其他 OS 未实现此 Linux 快照 API。 |
| `xcss::log/windows_rotating::Allocation::drop` | SDK 返回的 security descriptor / SID string 只用指定 `LocalFree` 释放一次；RAII 持有至所有借用结束。 | 原生 Windows 创建/打开/拒绝失败路径；不接受调用者任意 native 指针。 |
| `windows_rotating::sid_text` | 输入只来自活 token 或 SDK security descriptor。SID 转换成功后读取 SDK 分配的 NUL 结尾 UTF-16，最多 256 单元并复制为 owned String；allocation 全程活。 | ACL / owner 测试。SDK 返回存储有效性是 Windows API 的契约，未声称通过 Miri 验证 Win32。 |
| `windows_rotating::current_sid` | 成功的 OpenProcessToken 返回句柄一次移交 OwnedHandle；TokenUser 双调用仅接受 1..16 KiB 输出，u64 对齐缓存覆盖所需长度，读取其 SID 时 token / 缓存均活。 | 实际当前用户创建与 restricted token 拒读写。令牌 identity 不是产品字符串。 |
| `windows_rotating::descriptor` | 生成的 SDDL 仅使用 OS-derived 当前 SID 和固定 SYSTEM / Administrators，terminated UTF-16 与有效输出指针；成功 descriptor 交 RAII。 | 创建即 protected exact DACL，无继承 ACL 暴露窗口。不能授权任意额外身份。 |
| `windows_rotating::information` | 活 File 借用句柄，初始化输出 struct；检查 reparse、文件/目录类型与普通文件单 hardlink。 | 实际 junction / hardlink / 类型拒绝。SDK metadata 不能替代目录 namespace pins。 |
| `windows_rotating::verify_acl` | GetSecurityInfo 的 allocation 持有 owner / DACL / ACE。拒 NULL DACL、非 protected、错误 owner、未知/重复 ACE。先检查 ACE_HEADER 类型 / 固定字段长度，再形成较大 ACE 引用；SID revision/count/extent 校验后才调用转换。 | 原生坏 ACL 无修复、restricted token、链接拒绝。GetAce 在 OS 返回 descriptor 内的有效存储依赖 SDK 契约；未将外部原始字节作为 descriptor。 |
| `windows_rotating::open_handle` | NUL 终止 path、活 security attributes、固定 no-reparse flags，成功句柄仅一次交 File。路径祖先用不允许 DELETE sharing 的活 handles 固定；普通文件拒 hardlink。 | 原生 namespace 替换、现有不安全目录拒绝、容量/保留与 typed query 测试。相同 OS 身份的进程被明确视为可信，不声称隔离同 SID 攻击者。 |
| `windows_rotating::RotatingLogFile::create_private` | 已有父链先 pin；仅创建最终目录，生成 descriptor / attributes / terminated path 活至 CreateDirectoryW 返回。existing 对象验证而不修复。 | 真实原生日志默认 40 MiB 总保留、写失败可观察。未用 stderr 代替 SCM 持久文件。 |
| `windows_rotating_tests::changed_acl_poisoning_refuses_append_and_exposes_layer_failure` | owned fixture 的 terminated 路径借用至 SetNamedSecurityInfoW 返回，故意设置 NULL DACL，确认 writer 拒绝而不修复。没有产品授权入口。 | native ACL poisoning 后文件字节不变、typed sink / Layer failure 可观察。 |
| `windows_rotating_tests::actual_restricted_standard_token_cannot_read_or_write_private_logs` | 活 process token、固定 SID allocation 与 SID_AND_ATTRIBUTES 保持到 CreateRestrictedToken 返回；成功 token 一次交 OwnedHandle，再 impersonate 当前测试线程。restricted SID 与禁用管理员权限用于实际负例。 | 原生读写分别 PermissionDenied，恢复 identity 后 bytes 不变，无新文件。不是另一台机器的标准账户验收。 |
| `windows_rotating_tests::Impersonation::drop` | RAII 仅在成功 impersonation 后创建，失败或正常结束均 RevertToSelf，避免测试线程身份污染。 | Windows 原生测试实际执行。GNU 交叉检查只证明编译，不能替代 native 行为。 |

常规产品逻辑、通用 JSON、schema identity、同步 bridge 与可信 DDL helper 没有新增 unsafe。可直接使用安全 SDK/标准库的逻辑采用安全 API；这里保留的调用涉及无对应安全库接口的 OS 句柄/ACL、精确 OFD byte-range lock 或独占 native SQLite 限制。测试证明指定风险案例，未证明整个 OS、C SQLite 或第三方库无缺陷。
