# @xcss/web/contracts

唯一 `@xcss/web@1.0.0` 包的 `contracts` 公开子路径提供 xcss 当前跨语言通信合同：TypeScript 类型、针对 `unknown` 的严格
运行时校验、JSON Schema，以及 Rust/TypeScript 共用的正反测试夹具。

当前合同组包括：

- 管理员登录请求与严格的管理员会话；
- 统一错误响应结构；
- 第一版状态合同；
- 五字段发行身份；
- 第一版备份清单。

```ts
import {
  isAdministratorSession,
  isErrorEnvelope,
  isStateContract,
} from "@xcss/web/contracts";

const value: unknown = await response.json();
if (!isAdministratorSession(value)) {
  throw new Error("响应不符合当前管理员 Session 合同");
}
```

运行时校验严格核对对象的完整字段集合；标识符、40 位源码修订号、64 位小写 SHA-256、JavaScript
安全整数、非空数组、唯一维护锁、规范管理员名及 43 字符令牌均有明确边界。
`AdministratorLoginRequest` 只验证 1–64 字节可打印 ASCII 的“不可信候选值”；它可能仍含 `@` 等最终
身份不允许的可打印字符。真正的用户名规范化、身份准入、密码策略和散列验证由服务端的
`xcss::admin_auth` 执行。JSON 字段只有 `username`，不存在 `email` 别名。

类型断言、泛型、`as` 和 JSON Schema 文件本身都不会自动验证一个运行时值。产品必须在信任边界调用校验函数
或经过审计的结构验证器。产品业务 DTO、物理路径、资源排序与唯一性、旧字段别名、历史清单
读取器和迁移不属于本包。只能经 `package.json#exports` 导入，禁止深层引用 `src/` 或 `dist/` 私有路径。
