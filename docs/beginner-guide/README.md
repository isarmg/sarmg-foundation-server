# xcss 初学者学习指南

## 1. 这套教程解决什么问题

xcss 同时包含 Rust、TypeScript、React、Vite、CSS、JSON Schema、SQLite 和 Python 发布工具。初学者
最容易犯的错误不是语法错误，而是把共享 primitive 当成完整产品能力：例如认为一个 TypeScript type 已经
验证网络 JSON，认为通用 SQLite pool 已经保证文件安全，或者把共享管理员 Session 当成中央账户服务。

本教程从边界开始，再逐层进入源码、测试、发布和真实消费者。读完后应能：

- 解释为什么 xcss 是 build-time dependency 而不是在线服务；
- 正确区分管理面唯一 `admin` 角色与设备/Client/媒体资源等数据面概念；
- 把管理员 username、密码、Argon2id、token、same-origin、CSRF 组合进产品而不削弱规则；
- 区分 TypeScript type、runtime guard、JSON Schema、fixture 与 Rust serde validation；
- 解释 Schema fingerprint 为什么对 SQL bytes、排序和 metadata shape 都严格；
- 使用 `requestJson` 和 `createAdministratorApiClient` 时避免跨源、无界响应和认证竞态；
- 判断一项重复代码是否值得共享，以及怎样发布一个可复核的不可变版本。

## 2. 十一章路线

1. [项目定位、硬边界与目录](01-project-overview.md)
2. [固定工具链、安装与第一次验证](02-environment-and-first-validation.md)
3. [Rust 管理员认证、错误、Server Target 与 SQLite](03-rust-error-and-sqlite-basics.md)
4. [Contracts、Schema Identity、Guard 与 JSON Schema](04-contracts-types-guards-and-json-schema.md)
5. [HTTP Client、Admin Web 与 React 认证生命周期](05-http-client-request-lifecycle.md)
6. [Design Tokens 与真实消费者集成](06-design-tokens-and-consumer-integration.md)
7. [版本、Package、Release 与破坏性变更](07-versioning-publishing-and-breaking-changes.md)
8. [测试、调试、代码评审与新增共享能力](08-testing-debugging-and-contribution.md)
9. [供应链、安全事件与日常运维](09-supply-chain-security-and-operations.md)
10. [源码阅读路线、练习与术语表](10-reading-roadmap-and-glossary.md)
11. [统一 Server/Web 构建与开发热更新](11-embedded-web-build.md)

建议先顺序读 1～5；前端开发者再读 6，发布/维护人员继续读 7～9。第 10 章提供按问题查入口的索引和可
操作练习。
维护带 Web 的 Server 时还应阅读第 11 章，区分正式内嵌资源与开发热更新。

## 3. 一页架构速览

```text
Rust Server
├─ xcss::admin_auth        username/密码/token/header/cookie安全原语
├─ xcss::contracts         管理员、Error、State、Release、Backup wire类型
├─ xcss::error             machine error envelope
├─ xcss::schema_identity   driver-independent SQLite身份算法
├─ xcss::server_target     只允许x86_64-unknown-linux-gnu Server
└─ xcss::sqlite            SQLx连接与诊断adapter

React/Vite管理Web（Xczs除外）
├─ @xcss/web/contracts        type + runtime guard + Schema + fixture
├─ @xcss/web/http-client      同源、有界JSON transport
├─ @xcss/web/admin-web        内存Session、竞态安全client、React hook、Vite baseline
└─ @xcss/web/design-tokens    scoped CSS/TS primitive

### 4.1 管理面只有 Administrator

所有管理 Web 的 Session 都是：

```json
{
  "authenticated": true,
  "user_id": "一个有界标识符",
  "username": "admin",
  "role": "admin",
  "csrf_token": "43字符URL-safe token"
}
```

数据库通常无需 `role` 列，wire 中的 `admin` 是固定常量。设备 credential、xsoc 配对、移动端 API key、
摄像头凭据和资源字段 `role=primary/thumbnail` 仍可存在，但不是管理 RBAC。

### 4.2 Server 只有 AMD64 GNU/Linux

业务 Server 唯一 target 是 `x86_64-unknown-linux-gnu`。`xcss::server_target` 在编译期拒绝 ARM、musl、
Windows、macOS 和32位目标。这个限制不能误加到 xsoc Client、Android/iOS、移动FFI或其他客户端。

### 4.3 Web 统一，业务模块可保留明确边界

xsos、xszs、xcos、xscs 的管理 Web 位于 `web`，使用精确 React/Vite/TypeScript/Node基线。
Xczs 也采用 `web-react-admin`：登录、导航和页面骨架由 React/xcss 渲染，文件列表与上传控制器
保留原生 ES modules，并通过独占 DOM 区域组合。它与其他产品使用相同管理员 wire、username/密码/token
和 Server 端同源/CSRF 规则。

### 4.4 类型不等于验证

```ts
const value = await response.json() as AdministratorSession;
```

上面只骗过编译器。真正信任前必须运行 `isAdministratorSession(value)`。JSON Schema 也只有在调用一个
经过选择的 validator 时才执行；把 Schema 文件放在仓库里不会自动保护网络边界。

### 4.5 当前状态必须验证

当前格式仍需检查身份、实际结构和业务约束；错误输入明确拒绝，不能只信任metadata自报。

## 5. 第一次阅读源码前

先看根 `Cargo.toml` 和 `package.json`，确认公开组件与工具链；再读各 crate/package README和 `src`；随后
看测试，最后看 Python policy、CI、release。不要从 `target`、`node_modules` 或生成的 `dist` 反推公共API，
它们可能是缓存或已过期产物。

## 6. 学习过程中的安全约定

- 示例只使用 `.test` 域、随机测试 token 和临时目录，不使用真实账号、密码、数据库或生产路径。
- 不通过关闭证书校验、same-origin、CSRF、hash policy 或size limit来排障。
- 不在浏览器存储中保存 Session/CSRF；页面重载通过 HttpOnly Cookie和`restore()`重建内存状态。
- 不修改消费者 lock或发布资产来“配合”本地 sibling path；本地联调与最终不可变依赖是两个明确阶段。
- 本教程中的命令在代码全部完成后的统一验证阶段执行；修改源码时先保持工作树范围清晰。

## 7. 学成标准

你能画出一次管理员 login 从 raw headers、strict JSON、username/密码策略、Argon2、Session persistence到React
状态的完整路径；能解释每一层还缺什么产品责任；能用一个正例和至少四类负例评审新合同；能从真实tgz
而不是workspace import验证package；能指出为何Xczs、客户端多架构和xcss以服务端依赖库提供能力是合理差异；能在
不添加兼容代码的前提下设计一次新当前版本。
