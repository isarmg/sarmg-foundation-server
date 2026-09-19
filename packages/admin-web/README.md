# @sarmg/admin-web

`@sarmg/admin-web` 是非 Dufs Sarmg 管理 Web 的当前共享基线，提供以下公开入口：

- 根入口：`createAdministratorApiClient`、密码格式校验与客户端类型；
- `@sarmg/admin-web/react`：`useAdministratorSession` 认证状态机；
- 构建工具链与 TypeScript/Vite 配置由 `@sarmg/web-toolchain` 单独提供。

```ts
import { createAdministratorApiClient } from "@sarmg/admin-web";
import { useAdministratorSession } from "@sarmg/admin-web/react";

const administratorApi = createAdministratorApiClient();

export function App() {
  const auth = useAdministratorSession(administratorApi);
  // auth.phase: loading | anonymous | anonymous_logout_unconfirmed | authenticated | error
  return null;
}
```

认证入口为 `/api/v2/auth/login|session|logout`，`updateAccount()` 调用 `/api/v2/platform/administrators/self`，Session 必须严格匹配
`{authenticated:true,user_id,username,role:"admin",csrf_token}`。登录请求精确为 `{username,password}`；
username 是产品本地管理员名而不是邮箱。Web guard 只限制候选为 1～64 字节 printable ASCII；Server
仍须执行 trim/lower 与 canonical 准入，含 `@` 的候选不能成为管理员身份。认证状态和 CSRF 仅保存在
JavaScript closure 内存；绝不写入 `localStorage`、`sessionStorage`、IndexedDB 或 URL。login/restore/logout/updateAccount 按调用顺序串行，generation
机制阻止较旧响应覆盖新 Session；多个并发 restore 会复用同一 Promise；401 只清理发出该请求时仍有效的
Session。

浏览器中的 `baseUrl` 必须与页面 runtime origin 完全相同，业务 path 必须留在 `/api/v2/`。调用方不能
覆盖 `X-CSRF-Token`，也不能把 credential 改成 cross-origin。成功响应仍需调用方提供 runtime guard。

当前精确工具链为 Node `26.7.0`、React/React DOM `19.2.8`、Vite `7.3.6`、React plugin `4.7.0`、
TypeScript `5.8.3`、`@types/node` `26.6.2`、`@types/react` `19.2.18`、`@types/react-dom` `19.2.5`。版本范围、caret、tilde 或同一
工具链版本漂移由 `@sarmg/web-toolchain` 的精确断言拒绝。

Foundation 定义 Cookie、密码散列、Session、限流和安全审计；产品负责挂载、初始化和能力选择，以及业务 guard、
路由、页面、品牌和可访问性验收。本包没有 viewer/operator、多角色、SSO、token 持久化或旧 API fallback。

退出立即关闭本地授权。网络或服务器错误保留内存中的撤销目标，`logout()` 可主动重试；不无限重试、不写浏览器存储，也不恢复业务页面。只有确认注销或明确失效才清理上下文。
