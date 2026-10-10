# 模块索引

## 当前组件地图

| 层 | 内部模块或公开子路径 | 一句话用途 |
|---|---|---|
| Rust 安全 | `xcss::admin_auth` | 管理员名、Argon2id、令牌、Cookie、同源和 CSRF 原语 |
| Rust 认证 | `xcss::admin_core` | 账户/会话生命周期、登录限流、认证与安全审计机制 |
| Rust HTTP | `xcss::admin_axum` | Axum 管理员 HTTP 挂载、请求校验与响应适配 |
| Rust HTTP | `xcss::admin_hyper` | Hyper 管理员 HTTP 挂载与同等安全适配 |
| Rust 认证存储 | `xcss::admin_sqlite` | 持久管理员和会话的 SQLite 存储与事务 |
| Rust 静态认证 | `xcss::admin_static` | 单个静态管理员、当前账户文件与内存会话 |
| Rust 配置 | `xcss::config` | 分层配置、显式环境映射、来源与输入预算 |
| Rust 协议 | `xcss::contracts` | 管理员、状态、发行、备份和错误的严格通信格式 |
| Rust 错误 | `xcss::error` | 有界错误码、请求 ID 和结构化错误响应 |
| Rust 文件安全 | `xcss::fs_safety` | 描述符相对访问、链接拒绝、权限与身份验证 |
| Rust 日志 | `xcss::log` | 结构化事件、脱敏、筛选和有界轮转 |
| Rust 运维 | `xcss::operations` | 公共操作记录、审计与相应数据库机制 |
| Rust 公共数据库 | `xcss::platform_db` | 管理控制面的公共 SQLite 表与结构 |
| Rust 数据身份 | `xcss::schema_identity` | SQLite 规范结构指纹与四分量身份算法 |
| Rust 秘密 | `xcss::secret` | 秘密值的安全类型与处理原语 |
| Rust 秘密封装 | `xcss::secret_envelope` | 当前秘密封装格式与认证加密 |
| Rust HTTP 连接 | `xcss::secure_http` | 服务端出站 HTTP 的安全连接机制 |
| Rust CLI | `xcss::server_cli` | 机器错误、就绪身份核验和共同 HTTP 解析拒绝 |
| Rust 生命周期 | `xcss::server_runtime` | 服务启动、关闭、健康和就绪状态 |
| Rust 平台 | `xcss::server_target` | 目标常量；整个 crate 的 Linux AMD64 GNU 门禁位于根入口 |
| Rust SQLite | `xcss::sqlite` | SQLx 连接、PRAGMA、诊断、身份适配与只读校验副本 |
| Rust 状态文件 | `xcss::state_file` | 安全原子写、运行/维护锁与持久维护门 |
| Rust 测试 | `xcss::testkit` | 供测试使用的公共安全夹具与辅助机制 |
| Rust Web 资源 | `xcss::web_assets` | 确定性内嵌清单、SHA-256、HTTP 与开发目录提供器 |
| Web 合同 | `@xcss/web/contracts` | TypeScript 类型、运行时校验、JSON Schema 和夹具 |
| Web 传输 | `@xcss/web/http-client` | 同源、有界、可取消的 JSON 请求 |
| Web 认证 | `@xcss/web/admin-web` | 内存会话、竞态安全认证客户端与 React 钩子 |
| Web 管理外壳 | `@xcss/web/admin-shell` | 共享登录、导航、账号设置及业务区域组合 |
| Web 组件 | `@xcss/web/admin-ui` | 管理组件、可访问性、日期控件与内容块 |
| Web 样式 | `@xcss/web/design-tokens` | 限定作用域的设计和可访问性原语 |
| Web 字体 | `@xcss/web/web-fonts` | 随发行包提供的字体及预加载机制 |
| Web 构建 | `@xcss/web/web-toolchain` | 精确工具链、TypeScript/Vite 配置和服务端构建编排 |
| 发布 | Python tools/scripts | 软件包归档、状态/发行身份、release-tree 和工作流校验 |

以上 24 个 Rust 模块属于一个 crate，8 个 Web 入口属于一个 npm 包；不是 32 个独立发布的软件包。

## 目录逐层解释

```text
Cargo.toml           唯一 Rust package xcss 的身份、外部依赖与 lint
src/lib.rs           全 crate Linux AMD64 GNU 编译门禁与公开模块
src/<module>/        内部实现、fixture、单元测试（没有子 Cargo.toml）

package.json         唯一 @xcss/web 包的 exports、peer、engine 和 platform 约束
web/<module>/
├─ src/              TypeScript 源码
├─ test/             从根 dist/<module> 导入的测试
├─ scripts/          内部资源复制步骤
└─ schemas/fixtures/ contracts 的公开机器合同

dist/<module>/       根构建生成的统一输出，不是源码事实源

scripts/             用户/CI调用的稳定命令入口
tools/               policy/release/package的实现和负例
docs/                任务指南、开发说明与专题参考
```

为什么没有 `clients/`？因为 npm 软件包是被产品构建消费的库，不是本仓运行的产品客户端。为什么没有
`config/` 和 `deploy/`？因为 xcss 没有守护进程、systemd或运行配置。
