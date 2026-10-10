# @xcss/web/admin-web

`@xcss/web/admin-web` 提供 xcss 管理 Web 的共享认证基线，包括以下公开入口：

- 根入口：`createAdministratorApiClient`、密码格式校验与客户端类型；
- `@xcss/web/admin-web/react`：`useAdministratorSession` 认证状态机；
- 构建工具链与 TypeScript/Vite 配置由 `@xcss/web/web-toolchain` 单独提供。

```ts
import { createAdministratorApiClient } from "@xcss/web/admin-web";
import { useAdministratorSession } from "@xcss/web/admin-web/react";

const administratorApi = createAdministratorApiClient();

export function App() {
  const auth = useAdministratorSession(administratorApi);
  // auth.phase: loading | anonymous | anonymous_logout_unconfirmed | authenticated | error
  return null;
}
```

认证入口为 `/api/v1/auth/login|session|logout`，`updateAccount()` 调用 `/api/v1/platform/administrators/self`，会话必须严格匹配
`{authenticated:true,user_id,username,role:"admin",csrf_token}`。登录请求精确为 `{username,password}`；
`username` 是产品本地管理员名，而不是邮箱。Web 运行时校验只将候选值限制为 1～64 字节可打印 ASCII；服务端
仍须去除首尾空白、转换小写并检查规范身份，含 `@` 的候选不能成为管理员身份。认证状态和 CSRF 仅保存在
JavaScript 闭包内存中，绝不写入 `localStorage`、`sessionStorage`、IndexedDB 或 URL。
`login`、`restore`、`logout`、`updateAccount` 按调用顺序串行执行；代次机制阻止较旧响应覆盖新会话，
多个并发 `restore` 复用同一个 Promise；401 只清理发出该请求时仍有效的会话。

浏览器中的 `baseUrl` 必须与页面当前来源完全相同，业务路径必须留在 `/api/v1/`。调用方不能
覆盖 `X-CSRF-Token`，也不能把凭据发送到跨源地址。成功响应仍需调用方提供运行时校验函数。

当前精确工具链为 Node `26.7.0`、React/React DOM `19.3.0`、Vite `8.3.3`、React plugin `6.1.2`、
TypeScript `7.0.2`、`@types/node` `26.6.4`、`@types/react` `19.3.0`、`@types/react-dom` `19.3.0`。版本范围、插入符号范围（`^`）、波浪符号范围（`~`）或同一
工具链版本漂移由 `@xcss/web/web-toolchain` 的精确断言拒绝。

xcss 定义 Cookie、密码散列、会话、限流和安全审计；产品负责挂载、初始化和能力选择，以及业务校验、
路由、页面、品牌和可访问性验收。本包不提供观察员或操作员角色、多角色、SSO、令牌持久化或旧 API 回退。

退出立即关闭本地授权。网络或服务器错误保留内存中的撤销目标，`logout()` 可主动重试；不无限重试、不写浏览器存储，也不恢复业务页面。只有确认注销或明确失效才清理上下文。
