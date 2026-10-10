# xcss 初学者学习指南

## 1. 这套教程解决什么问题

xcss 包含 Rust、TypeScript、React、Vite、CSS、JSON Schema、SQLite 和 Python 发布工具。本教程先解释
共享库的责任边界，再介绍源码、测试、发布和真实消费者。类型声明不能代替网络输入验证，通用 SQLite
连接池不能代替文件安全校验，共享管理员会话也不意味着所有产品连接同一个账户服务。

读完后应能：

- 解释为什么 xcss 是构建期依赖；
- 区分管理面唯一的 `admin` 角色与设备、客户端及媒体资源等数据面概念；
- 正确组合管理员名、密码、Argon2id、令牌、同源检查和 CSRF；
- 区分 TypeScript 类型、运行时校验、JSON Schema、测试夹具和 Rust 反序列化验证；
- 解释结构指纹为何严格绑定 SQL 字节、排序和元数据结构；
- 使用 `requestJson` 和 `createAdministratorApiClient` 避免跨源请求、无界响应及认证竞态；
- 判断哪些能力适合共享，并发布可复核的不可变版本。

## 2. 十一章路线

1. [项目定位、硬边界与目录](01-project-overview.md)
2. [固定工具链、安装与第一次验证](02-environment-and-first-validation.md)
3. [Rust 管理员认证、错误、服务端编译目标与 SQLite](03-rust-error-and-sqlite-basics.md)
4. [合同、结构身份、运行时校验与 JSON Schema](04-contracts-types-guards-and-json-schema.md)
5. [HTTP 客户端、管理 Web 与 React 认证生命周期](05-http-client-request-lifecycle.md)
6. [设计令牌与真实消费者集成](06-design-tokens-and-consumer-integration.md)
7. [版本、软件包、发行与破坏性变更](07-versioning-publishing-and-breaking-changes.md)
8. [测试、调试、代码评审与新增共享能力](08-testing-debugging-and-contribution.md)
9. [供应链、安全事件与日常运维](09-supply-chain-security-and-operations.md)
10. [源码阅读路线、练习与术语表](10-reading-roadmap-and-glossary.md)
11. [统一服务端与 Web 构建与开发热更新](11-embedded-web-build.md)

建议先顺序读第 1～5 章；前端开发者继续读第 6 章，发布和维护人员继续读第 7～9 章。第 10 章提供源码
索引及练习；维护带 Web 的服务端时还应阅读第 11 章，理解正式内嵌资源与开发热更新的区别。

## 3. 一页架构速览

```text
Rust 服务端
├─ xcss::admin_auth        管理员名、密码、令牌、请求头和 Cookie 安全原语
├─ xcss::contracts         管理员、错误、状态、发行和备份通信类型
├─ xcss::error             统一机器错误响应
├─ xcss::schema_identity   不依赖数据库驱动的 SQLite 身份算法
├─ xcss::server_target     只允许 x86_64-unknown-linux-gnu 服务端
└─ xcss::sqlite            SQLx 连接与诊断适配器

React/Vite 管理 Web
├─ @xcss/web/contracts        类型、运行时校验、结构定义和测试夹具
├─ @xcss/web/http-client      同源且有界的 JSON 传输
├─ @xcss/web/admin-web        内存会话、认证客户端与 React 钩子
└─ @xcss/web/design-tokens    限定作用域的 CSS/TypeScript 设计原语
```

## 4. 必须先理解的边界

### 4.1 管理面只有管理员

所有管理 Web 的会话使用以下结构：

```json
{
  "authenticated": true,
  "user_id": "一个有界标识符",
  "username": "admin",
  "role": "admin",
  "csrf_token": "43字符URL-safe token"
}
```

数据库通常无需 `role` 列，通信格式中的 `admin` 是固定常量。设备凭据、xsoc 配对、移动端 API 密钥、
摄像头凭据及资源字段 `role=primary/thumbnail` 属于数据面，不构成管理 RBAC。

### 4.2 服务端只有 AMD64 GNU/Linux

业务服务端唯一编译目标是 `x86_64-unknown-linux-gnu`。`xcss::server_target` 在编译期拒绝 ARM、musl、
Windows、macOS 和 32 位目标。该限制针对服务端，不能误用到客户端或移动端。

### 4.3 管理 Web 统一，业务模块保持产品边界

xsos、xszs、xcos、xscs 的管理 Web 位于 `web`，使用精确的 React/Vite/TypeScript/Node 基线。
Xczs 同样采用 `web-react-admin`：登录、导航和页面骨架由 React/xcss 渲染；文件列表及上传控制器
保留原生 ES 模块，通过独占 DOM 区域组合。各产品使用相同的管理员通信合同、密码与令牌规则、同源检查和 CSRF。

### 4.4 类型不等于验证

```ts
const value = await response.json() as AdministratorSession;
```

上面的类型断言只改变编译器的看法。真正信任响应前必须调用 `isAdministratorSession(value)`。
JSON Schema 也只有被选定的验证器执行时才生效；把结构文件放进仓库不会自动保护网络边界。

### 4.5 当前状态必须验证

当前格式仍须检查身份、实际结构和业务约束。错误输入应明确拒绝，不能只信任元数据中的自报身份。

## 5. 第一次阅读源码前

先看根 `Cargo.toml` 和 `package.json`，确认单体身份、公开模块及工具链；再看各模块说明和源码，随后
阅读测试、Python 规则、CI 及发行工具。`target`、`node_modules` 和生成的 `dist` 可能含缓存或过期产物，
不能用来反推公共 API。

## 6. 学习过程中的安全约定

- 示例只使用 `.test` 域、随机测试令牌及临时目录，不使用真实账号、密码、数据库或生产路径。
- 排障时保留证书验证、同源检查、CSRF、散列规则和大小限制。
- 会话及 CSRF 不写入浏览器存储；页面重载后通过 HttpOnly Cookie 和 `restore()` 重建内存状态。
- 本地联调与最终不可变依赖分别验证，不修改消费者锁文件或正式资产来适配同级路径。
- 修改源码时保持工作树范围清晰；完成后执行教程中的验证命令。

## 7. 学成标准

你应能画出一次管理员登录从原始请求头、严格 JSON、用户名和密码规则、Argon2、会话持久化到 React
状态的完整路径，说明每层仍由产品承担哪些责任；能用一个正例和至少四类负例评审合同；能从真实发行
归档验证软件包；能解释产品业务模块与服务端平台约束的边界，并在不增加兼容分支的前提下设计新版本。
