# @sarmg/contracts

`@sarmg/contracts@0.5.0` 发布 Sarmg 当前跨语言 wire contract：TypeScript 类型、针对 `unknown` 的严格
runtime guard、JSON Schema，以及 Rust/TypeScript 共用的正反 fixture。

当前合同组包括：

- 管理员登录请求与严格 Administrator Session；
- Error Envelope；
- State Contract v1；
- 五字段 Release Identity；
- Backup Manifest v2。

```ts
import {
  isAdministratorSession,
  isErrorEnvelope,
  isStateContract,
} from "@sarmg/contracts";

const value: unknown = await response.json();
if (!isAdministratorSession(value)) {
  throw new Error("响应不符合当前管理员 Session 合同");
}
```

Guard 对对象字段执行 exact-key 检查；identifier、40 位 source revision、64 位小写 SHA-256、JavaScript
safe integer、非空数组、唯一 maintenance lock、canonical 管理员 username 和 43 字符 token 均有明确边界。
`AdministratorLoginRequest` 只验证 1–64 字节 printable ASCII 的“不可信候选值”；它可能仍含 `@` 等最终
身份不允许的可打印字符。真正的 username 规范化、canonical 准入、密码策略和散列验证由 Server 的
`sarmg-admin-auth` 执行。JSON 字段只有 `username`，不存在 `email` alias。

类型断言、泛型、`as` 和 JSON Schema 文件本身都不会自动验证一个运行时值。产品必须在信任边界调用 guard
或经过审计的 Schema validator。产品业务 DTO、物理路径、资源排序/唯一性、旧字段 alias、历史 manifest
reader 和迁移不属于本包。只能经 `package.json#exports` 导入，禁止深层引用 `src/` 或 `dist/` 私有路径。
