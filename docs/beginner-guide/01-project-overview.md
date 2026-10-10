# 01. 项目定位、硬边界与目录

## 1.1 为什么需要 xcss

多个产品都会遇到“看起来很基础”的问题：管理员密码怎样散列、会话令牌怎样编码、浏览器如何严格
同源、错误 JSON 长什么样、SQLite Schema怎样绑定产品身份、React/Vite用哪个精确版本、发行树怎样证明
没有被替换。如果每个仓库都独立实现，几个月后通常会产生不同长度、不同错误、不同回退和不同测试。

xcss 按职责和通用性共享服务端基础能力。即使暂时只有一个产品使用，产品中立、可独立测试的机制也可以
进入上游；产品专有业务不因代码相似而上移。业务页面、业务数据库、外部设备和数据面协议仍由产品负责，
公共认证、运行时、文件安全和管理 UI 的机制由 xcss 提供。

## 1.2 构建期模型

```text
开发/CI时
产品源码 -> 锁定xcss commit/tgz -> 编译/打包 -> 产品制品

生产运行时
用户/客户端 -> 产品制品
                  X 不连接xcss
                  X 不访问npm registry
                  X 不依赖GitHub在线
```

这带来两个重要结果：

1. xcss 仓库或软件包注册中心故障不会影响已经部署的产品；
2. xcss 修复不会自动进入生产，每个消费者必须更新精确依赖、重建、验证和重新发布。

它与“中央身份平台”完全不同。共享的管理员认证是库和通信格式合同，不是所有产品登录同一个账户数据库。

## 1.3 当前组件地图

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

## 1.4 目录逐层解释

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
docs/                仅五类中文文档
```

为什么没有 `clients/`？因为 npm 软件包是被产品构建消费的库，不是本仓运行的产品客户端。为什么没有
`config/` 和 `deploy/`？因为 xcss 没有守护进程、systemd或运行配置。

## 1.5 “统一”到底统一什么

当前跨产品统一：

- 管理员只有 `admin` 角色；
- 三个管理员认证路径和会话 JSON；
- 管理员 username、密码、Argon2id、令牌、Origin/Host/Sec-Fetch-Site、CSRF 安全原语；
- 错误响应结构和State/Release/Backup合同；
- SQLite metadata/fingerprint；
- 服务端唯一target；
- 所有 React 管理 Web（包括 Xczs）的 React/Vite/Node/TypeScript；
- package/release/workflow最低供应链规则。

仍由产品决定：

- 产品管理员初始化入口、HTTP 挂载和持久或静态运行形态选择；管理员表、会话、Cookie、TTL、登录限流及安全审计机制由 xcss 固定；
- 设备/Client/API key/摄像头/媒体令牌等数据面身份；
- 业务 route、DTO、业务数据库表和事务、业务锁及外部副作用；
- 专有业务页面、组件、品牌、业务主题状态及文件/媒体流；公共管理外壳、组件、令牌和字体由 xcss 提供；
- systemd、反向代理、产品配置字段和 Secret 来源、业务 backup/restore 及发行强化规则；公共配置、秘密、文件安全和维护锁机制由 xcss 提供。

## 1.6 唯一管理员角色的正确理解

“仅保留管理员”只针对管理控制面。以下两个字段虽然也叫 role，却不是管理RBAC：

- HTML/ARIA `role="alert"`：可访问性语义；
- Media资源 `role="primary"`/`"thumbnail"`：媒体对象用途。

这些不需要删除。反过来，数据库管理用户的 viewer/operator/role列、Web按管理角色分支和额外管理权限
路径则不属于当前合同。

## 1.7 AMD64边界的正确理解

业务服务端 binary 只声明 `xcss` 依赖。整个 crate 在根入口检查架构、OS 和 libc，`xcss::server_target` 还提供目标常量；只接受：

```text
x86_64 + linux + gnu + 64-bit
```

全部内部模块共享这个编译边界，选择某个模块不会改变整个 crate 的平台要求。

## 1.8 Xczs 的原生业务模块

Xczs 当前采用 `web-react-admin`：登录、导航和页面骨架使用 React/xcss；文件列表及上传控制器保留原生
ES 模块，通过独占 DOM 区域与共享外壳组合。全部正式 Web 资源与 Rust 可执行文件一起编译。
服务端登录仍返回 `AdministratorSession`，密码、令牌、同源检查和 CSRF 使用相同的 Rust 原语。
业务模块的实现方式不改变共享认证与构建合同。

## 1.9 仅支持当前格式生命周期

```text
开发期改变合同
 -> 直接改当前实现/Schema/fixture/消费者
 -> 重建测试状态
 -> 不保存另一套读取器
```

“拒绝非当前”是正常安全行为，不应通过多试几个hash、忽略未知 field或自动建库来提高“兼容性”。

## 1.10 本章练习

1. 在根两个清单中核对单体身份，列出内部 Rust 模块与 Web 子路径，并解释它们如何共同发布。
2. 从一个产品中找出管理员身份与数据面凭据，解释为何二者不能合并。
3. 画出产品build时与production runtime时xcss是否在线的两张图。
4. 解释 Xczs 如何组合 React 管理外壳与原生文件业务模块，以及客户端多架构为何不是服务端编译目标例外。
5. 从功能台账任选一个“保障”项，写出删除后的具体攻击或故障路径。
