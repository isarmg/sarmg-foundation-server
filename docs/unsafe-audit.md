# 原生 unsafe 审查

本次审查针对 xcss 自有 Rust 源码。第三方库内部的 unsafe 不算作本仓实现；依赖的来源和版本由根 `Cargo.toml` 与 `Cargo.lock` 控制。

## 默认规则

唯一根 crate 使用根 `Cargo.toml` 的 `[lints.rust]` 和 `[lints.clippy]`：`unsafe_code = deny`、`unsafe_op_in_unsafe_fn = deny`、`clippy::undocumented_unsafe_blocks = deny`。允许例外的最小范围是下面列出的原生函数；业务组合、日志轮转和权限策略本身没有 unsafe 豁免。每个块的 `SAFETY` 注释必须说明指针有效性、所有权、生命周期或线程独占前提。

## 保留的接口

| 位置 | 必要性 | 安全前提与验证 |
|---|---|---|
| `src/sqlite/native_limits.rs::native_limit` | SQLx 0.9 的安全接口没有提供这四项 `sqlite3_limit` 设置和读回；SQL 文本不能等价限制值大小、VM 操作和绑定参数数量。 | 使用 SQLx 独占 `LockedSqliteHandle` 暂停工作线程；固定类别与范围已验证；同一锁定 `libsqlite3-sys` 链接身份；不保存句柄。已有测试验证本连接限制、其他连接不受影响、无效输入不修改状态以及替换连接同样受限。 |
| `src/sqlite/native_security.rs::set_and_verify` | `SQLITE_DBCONFIG_DEFENSIVE` 没有等效 defensive PRAGMA，也没有 SQLx 安全设置 API。 | 独占活连接；C variadic 参数使用 `c_int` 和有效输出指针；只允许设置 1 和读取 -1；两次返回码和最终状态都检查。已有测试验证拒绝 schema 写入并保留普通 SQL。 |
| `src/sqlite/validation_snapshot.rs::lock_read` | SQLite 快照需要特定字节区间的 Linux OFD 读锁。Rust `File` 的整文件 flock 和 rustix 的 process-associated `fcntl_lock` 不等价，不能替换锁协议。 | File 持有 fd，flock 全字段初始化，OFD 要求 PID 为 0；非阻塞、区分 busy 与不可用；持锁文件生命周期覆盖复制。Linux 原生行为由该模块现有风险测试覆盖，本轮 macOS 不能执行这些测试。 |
| `src/log/windows_rotating.rs` 中的原生函数与 `Allocation::drop` | 标准库与现有 rustix Windows 接口不能核验精确 DACL、protected 私有锚及服务继承，或实现不共享删除的目录 pin。 | 路径 NUL 检查、宽字符串保持有效；token/security descriptor 的 API 分配通过唯一 RAII 所有者释放；令牌 buffer 按 u64 对齐、最小 TOKEN_USER 大小与 16 KiB 上限验证；ACE 类型/固定字段/SID 可变范围检查；File/OwnedHandle 各关闭一次。所有 unsafe 仅允许于实际 FFI 函数，取消整个模块豁免。 |
| `src/log/windows_rotating_tests.rs` 原生测试夹具、权限测试与清理函数 | 真实受限令牌、DACL 变更和线程 impersonation 无安全标准库替代；模拟结果无法覆盖这些风险。 | 固定 SID、成功后接管句柄、LocalFree RAII、线程恢复运行时校验；测试只修改自己的临时对象。xcss 整个 crate 仅支持 Linux AMD64，表内 Windows 源码属于历史审查记录，在当前支持目标下不可达。 |

本轮补全 Windows 每个原生块的安全前提，并拒绝不足以包含 TOKEN_USER 的 OS 输出大小；没有删除权限、文件锁、范围校验或有效风险测试来消除 unsafe。SQLite 单元测试按职责移到同模块 `tests.rs`，公共 API 与外部包名保持不变。

## 本轮验证与边界

Rust 1.99.0 macOS 已执行更新后的认证、错误、合同、秘密封装、Schema、SQLite 与日志八个 crate 的测试，66 项通过。Python 工具测试与 xcss 一致性检查另行执行。Windows 日志和 Linux 完整工作区已通过包含测试源码的交叉 clippy；Web 包构建、类型、73 项单元测试、Chromium/Firefox 的 44 项真实浏览器测试和八个发行归档的隔离安装及 41 个公开导出入口冒烟验证通过。

完整 Linux 工作区测试和 Windows 原生日志测试须在对应系统执行。本文件不以交叉检查或本地冒烟验证代替正式发行及真实目标系统验收。

## 0.11.3 Windows 服务日志权限补丁

`src/log/windows_rotating.rs` 的共享 descriptor/ACL 读取与验证增加 `WindowsLogAccess` 策略；产品仍不复制原生日志或 ACL 实现。SDDL 仅由固定 owner、固定权限和长度不超过 64 的规范 SCM SID 组成，不接受任意用户或表达式。原生 allocation、SID、ACL 和句柄继续由既有 RAII 管理，指针借用不超过 descriptor 生命周期。每个 ACE 在转为 SID 前检查 header、类型和变长大小，再逐项核对 trustee、mask 与继承标志；保留 READ_CONTROL 而不授予 owner 隐式 WRITE_DAC。

服务角色与现有当前用户角色使用同一轮转和防重解析实现。LocalService owner、service SID 限定权限及 OWNER RIGHTS 是实际 xsoc MSI 合同，不通过产品发行版本推断。安全替代是消费此共享策略接口；标准文件 API 无法声明或校验上述 Windows DACL，不能通过弱化权限取得零 unsafe。

新增原生测试只在临时树调整故意不安全的 ACL，并在受限令牌下实际申请数据读取与 WRITE_DAC。另从已打开的真实文件取得 owner/group/DACL，读取实际线程 impersonation 令牌并核对其用户就是文件 owner；使用文件权限映射执行 AccessCheck，独立证明 READ_CONTROL 授予而 WRITE_DAC 和 FILE_READ_DATA 拒绝，避免路径遍历及文件打开条件掩盖 OWNER RIGHTS。原生输出缓冲区按类型对齐、长度受限，descriptor 与令牌保持有效；impersonation 运行时校验在清理前恢复线程身份。GNU 严格的 Clippy 证明目标源码边界；真实 Windows 执行和 SCM 角色分别等待 xcss/xsoc 最终 Source CI。

## 0.11.4 认证私有锚与继承

生产层未新增 NT ABI、动态符号、EventLog 或产品专用分支。既有 CreateDirectoryW/CreateFileW FFI 使用受控 NULL attributes 创建服务子目录/新叶；父目录必须在同一调用前经过完整认证，并由禁共享删除的句柄保持。`LeafOpen` 区分 Existing、Protected 与 Inherited，使 NULL descriptor 不会混淆 OPEN_EXISTING/CREATE_NEW。精确四项 trustee/mask 与有效 flags 校验保持，锚索引固定，不在后续操作重新发现更远的安全锚；最终 LocalService owner 每次重新核对。

新增 test-only `FixtureOwnerToken` 仅复制真实 CI 令牌，改变该副本的默认 TokenOwner 为真实 TokenUser，并在创建前 impersonate；身份、群组与进程令牌不变。SDK SID allocation 及令牌唯一 RAII 持有，运行时校验先 RevertToSelf 再关令牌，所有原生块有局部 SAFETY 注释。初建时设置 owner 避免 OWNER-only 后改物理对象可能改变 OWNER RIGHTS 的测试干扰。受限 owner 对照同时覆盖 protected 和继承文件；不能用交叉类型检查证明实际 Windows 授权。

属主变异负测只将自己临时树的后代隔离为合法 protected 精确 DACL，先核验完整命名空间，再变更目标属主并复位目标 DACL；不让父权限继承传播混淆属主拒绝原因，真实继承用例保持独立覆盖。SDK 物理属主/DACL 诊断只在断言失败时读取已持有句柄，安全描述符与转换后的 SDDL 字符串分别由唯一 LocalFree RAII 管理，输出按 SDK 长度读取且不超过 16,384 个 WCHAR，不读取业务内容。

正式发行要求原生 Windows 行为和真实 LocalService 生命周期由最终 CI/消费者 Source 分别实证，Linux 工作区、Web 与正式制品验证也在发行门禁执行。历史发行与验证记录不代替本版继承模式的验收。

## 当前单体边界

xcss 目前为单个 Linux AMD64 GNU crate，Windows 日志源码在该 target 下不可达。上述 macOS/Windows、多个 crate 和多个发行归档的数量记录属于合并前历史验收事实，不代表当前单体已执行相同平台验证；当前发布需按新版 CI 对整 crate 和唯一 npm tgz重新验收。
