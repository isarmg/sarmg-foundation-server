# 01. 项目定位、硬边界与目录

## 1.1 为什么需要 Foundation

多个产品都会遇到“看起来很基础”的问题：管理员密码怎样散列、Session token怎样编码、浏览器如何严格
同源、错误 JSON 长什么样、SQLite Schema怎样绑定产品身份、React/Vite用哪个精确版本、发行树怎样证明
没有被替换。如果每个仓库都独立实现，几个月后通常会产生不同长度、不同错误、不同fallback和不同测试。

Foundation 的目标不是消灭所有重复，而是共享那些已经被至少两个真实产品证明具有相同语义的最小能力。
共享收益必须大于耦合成本；页面、业务数据库、外部设备、进程生命周期和数据面协议仍留在产品。

## 1.2 Build-time 模型

```text
开发/CI时
产品源码 -> 锁定Foundation commit/tgz -> 编译/打包 -> 产品制品

生产运行时
用户/客户端 -> 产品制品
                  X 不连接Foundation
                  X 不访问npm registry
                  X 不依赖GitHub在线
```

这带来两个重要结果：

1. Foundation 仓库或 registry 故障不会影响已经部署的产品；
2. Foundation 修复不会自动进入生产，每个消费者必须更新精确依赖、重建、验证和重新发布。

它与“中央身份平台”完全不同。共享的管理员认证是库和wire合同，不是所有产品登录同一个账户数据库。

## 1.3 当前组件地图

| 层 | 组件 | 一句话用途 |
|---|---|---|
| Rust安全 | `sarmg-admin-auth` | canonical 管理员 username、当前 Argon2id、token、Cookie、同源和 CSRF |
| Rust协议 | `sarmg-contracts` | strict管理员/状态/发布/备份/错误wire类型 |
| Rust错误 | `sarmg-error` | 有界machine code/request ID和Error Envelope |
| Rust数据 | `sarmg-schema-identity` | 无driver的SQLite Schema身份和fingerprint |
| Rust平台 | `sarmg-server-target` | Server编译只允许GNU/Linux AMD64 |
| Rust连接 | `sarmg-sqlite` | SQLx连接PRAGMA、诊断和identity adapter |
| Web合同 | `@sarmg/contracts` | TypeScript type、runtime guard、JSON Schema、fixture |
| Web传输 | `@sarmg/http-client` | 同源、有界、可取消的JSON请求 |
| Web认证 | `@sarmg/admin-web` | 内存Session、竞态安全auth client、React hook和Vite baseline |
| Web样式 | `@sarmg/design-tokens` | scoped设计和可访问性primitive |
| 发布 | Python tools/scripts | package tar、state/release identity、release-tree和workflow policy |

## 1.4 目录逐层解释

```text
rust/crates/<name>/
├─ Cargo.toml        package身份与精确内部依赖
└─ src/lib.rs        public API、实现和单元测试

packages/<name>/
├─ package.json      exports、peer、engine与build/test入口
├─ src/              TypeScript源码
├─ test/             从已构建dist导入的测试
├─ scripts/          clean/copy等确定性步骤
├─ schemas/fixtures/ 仅contracts所有
└─ dist/             生成物，不是手工事实源

consumers/
├─ consumer-matrix.json
└─ consumer-matrix.schema.json

scripts/             用户/CI调用的稳定命令入口
tools/               policy/release/package的实现和负例
docs/                仅五类中文文档
```

为什么没有 `clients/`？因为 npm package 是被产品构建消费的库，不是本仓运行的产品客户端。为什么没有
`config/` 和 `deploy/`？因为 Foundation 没有daemon、systemd或运行配置。

## 1.5 “统一”到底统一什么

当前跨产品统一：

- 管理员只有 `admin` 角色；
- 三个管理员auth路径和Session JSON；
- 管理员 username、密码、Argon2id、token、Origin/Host/Sec-Fetch-Site、CSRF 安全原语；
- Error Envelope和State/Release/Backup合同；
- SQLite metadata/fingerprint；
- Server唯一target；
- 非Dufs管理Web的React/Vite/Node/TypeScript；
- package/release/workflow最低供应链规则。

仍由产品决定：

- 管理员表、Session表、Cookie、TTL、登录限流、审计；
- 设备/Agent/API key/摄像头/媒体token等数据面身份；
- 业务route、DTO、数据库表、transaction、锁和外部副作用；
- 页面、组件、品牌、主题状态、文件/媒体stream；
- systemd、reverse proxy、配置、Secret、backup/restore和release强化规则。

## 1.6 唯一管理员角色的正确理解

“仅保留管理员”只针对管理控制面。以下两个字段虽然也叫 role，却不是管理RBAC：

- HTML/ARIA `role="alert"`：可访问性语义；
- Media资源 `role="primary"`/`"thumbnail"`：媒体对象用途。

这些不需要删除。反过来，数据库管理用户的 viewer/operator/role列、Web按管理角色分支和额外管理权限
路径则不属于当前合同。

## 1.7 AMD64边界的正确理解

`sarmg-server-target` 应被业务Server binary直接依赖。它检查架构、OS、libc和pointer width；只接受：

```text
x86_64 + linux + gnu + 64-bit
```

Host Monitor Agent仍需在Windows/macOS/Linux运行，Media有Android/iOS客户端，Foundation本身发布源码包，
Upgrade是离线CLI；它们不是“Server”，不能为了字面统一全部依赖该crate。

## 1.8 Dufs前端例外

Dufs原生ES modules与Rust binary一起嵌入，文件管理交互和测试体系已经围绕这一形态建立。把它迁到React
只会增加重写风险，不会改善统一认证的核心目标。因此Dufs保留原生前端，但Server登录仍返回Foundation
AdministratorSession，密码/token/same-origin/CSRF仍用同一Rust primitive。例外必须是可解释的产品边界，
不是随意漂移。

## 1.9 current-only生命周期

```text
开发期改变合同
 -> 直接改当前实现/Schema/fixture/消费者
 -> 重建测试状态
 -> 不保存另一套reader

稳定发布后未来需要状态转换
 -> 在线产品只接受target current
 -> sarmg-upgrade离线识别精确source
 -> 原子转换/验证到精确target
```

“拒绝非当前”是正常安全行为，不应通过多试几个hash、忽略unknown field或自动建库来提高“兼容性”。

## 1.10 本章练习

1. 在根manifest中列出6个crate和4个package，并为每个写一个“不负责”项。
2. 从一个产品中找出管理员身份与数据面credential，解释为何二者不能合并。
3. 画出产品build时与production runtime时Foundation是否在线的两张图。
4. 解释Dufs为何是前端例外、客户端多架构为何不是Server target例外。
5. 从功能台账任选一个“保障”项，写出删除后的具体攻击或故障路径。
