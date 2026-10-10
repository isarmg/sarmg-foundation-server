# 排查问题

先确认失败发生在源码检查、Rust 构建、Web 构建、发行归档还是产品运行阶段，并保留具体错误码与命令输出。

| 现象 | 检查与下一步 |
|---|---|
| 目标平台编译失败 | xcss 的 Rust 与 Web 构建需要 Linux x86_64 GNU；确认构建主机和 Cargo target |
| 清单/锁文件不一致 | 核对精确工具链和变更的依赖字段，按相同版本更新对应锁文件 |
| 产品来源检查失败 | 同时检查 `xcss-product.toml`、Cargo/npm 清单及锁文件的版本、完整 revision、tgz URL 和完整性摘要 |
| `links=sqlite3` 冲突 | 用 `cargo tree --locked` 找到不同 SQLite 原生链接来源，统一到当前 SQLx 适配器的依赖图 |
| Web 导出或静态资源缺失 | 重新运行公共构建，核对源文件到 dist、exports 和真实 tgz 的对应关系 |
| Web 资源预算超限 | 定位输出文件大小，拆分或优化内容；需要调整时使用有界 `maxAssetBytes` |
| `release_verified=false` | 查看报告中的发行清单状态，准备实际制品后运行 `verify-release --require-published` |
| 内嵌资源与页面不一致 | 对比实际二进制 `web-assets` 输出和本次 dist，重新执行共同构建 |

## 认证请求

| 现象 | 检查与下一步 |
|---|---|
| Origin/Host/Fetch Metadata 缺失或重复 | 检查真实浏览器与代理保留的请求头；普通 HTTP 管理请求使用完整同源头集合 |
| Origin 与 Host 不一致 | 核对协议、主机、有效端口和外部地址，修正可信代理的 Host 传递 |
| CSRF 不匹配 | 恢复当前会话并核对 Web 构建；检查旧请求和代理缓存 |
| 退出未确认 | 使用“重试退出”，等待服务端撤销确认 |
| 修改请求超时 | 按操作 ID 查询业务结果，再决定是否重新提交 |

WebSocket 握手使用 `require_administrator_websocket_origin`：完整 Origin/Host 仍需验证，Fetch Metadata 可缺失，存在时须为唯一 `same-origin`。认证和业务授权在握手来源检查之外执行。

## 数据和文件

| 现象 | 检查与下一步 |
|---|---|
| 私有目录或文件打开失败 | 检查绝对物理路径、属主、权限、链接和实际运行身份 |
| 数据库身份/结构指纹不匹配 | 保留原件，对比当前产品结构与实际 DDL，按产品的数据处理流程操作 |
| checkpoint busy | 检查仍在运行的 reader/writer，在产品维护窗口重试 |
| `PublishedDurabilityUnknown` | 已发生发布而目录同步失败，先检查实际状态，再按产品恢复策略处理 |
| 日志拒绝计数增长 | 检查字段预算、记录形状、目录权限和输出 sink 错误 |

完整条件见[文件参考](filesystem-handles.md)、[数据库与合同](reference/contracts-and-schema.md)、[配置和日志](configuration-cli-logging.md)。提交问题时附平台、版本、最小复现和脱敏输出。
