# 05. HTTP Client、Admin Web 与 React 认证生命周期

## 5.1 三层关系

```text
@sarmg/http-client
└─ requestJson：URL、credential、CSRF、timeout、body budget、Error Envelope

@sarmg/admin-web
├─ createAdministratorApiClient：auth路径、内存Session、竞态、业务guard
├─ /react：useAdministratorSession
└─ /vite：React/Vite配置

产品clients/web
└─ 页面、路由、业务API guard、品牌、错误文案、媒体/文件transport
```

理解这三层可以避免把通用transport当auth store，或把auth client当完整UI框架。

## 5.2 `requestJson`的完整请求流程

```text
调用requestJson(url, options)
├─ 校验timeout和response预算是safe integer且在范围内
├─ 解析baseUrl和target URL
│  ├─ 只允许HTTP(S)
│  ├─ 拒绝userinfo/control
│  └─ origin精确相同
├─ redirect固定error
├─ 合并Headers，默认Accept JSON
├─ unsafe method才注入有效csrfToken
├─ 合并caller AbortSignal与timeout，first-wins
├─ fetch，credentials默认same-origin
├─ 非2xx：有界读取并严格解析Error Envelope
└─ 2xx：处理empty response或有界读取/UTF-8/JSON parse
```

默认timeout 10000ms，允许1～120000ms。默认成功响应2MiB，配置硬上限64MiB；错误正文固定64KiB。

## 5.3 URL与redirect

同源比较用标准URL的`origin`，因此scheme、host、effective port都参与。`https://a.test`与
`https://a.test:443`同源，但`http://a.test`、`https://b.test`或`:444`不同源。URL userinfo如
`https://name:secret@a.test`直接拒绝。

即使初始URL同源，自动跟随30x也可能到第三方，所以`redirect`强制`error`。调用方不能传`follow`或
`manual`改变这一边界。

## 5.4 CSRF注入

safe methods是GET、HEAD、OPTIONS、TRACE；其他method视为unsafe。只有caller提供`csrfToken`且method
unsafe时才写`X-CSRF-Token`。token在http-client层只要求非空单行；admin-web提供的是已通过Session guard
的canonical 43字符token，Server再执行严格shape/hash比较。

为何不向GET注入？减少token暴露面并保持safe请求可缓存/诊断语义。为何不由http-client读取storage？因为
Session所有权属于更高层，持久存储本身不符合当前安全模型。

## 5.5 Timeout与caller abort

内部AbortController接收两个来源；第一个成功abort的来源决定错误code。timeout得到`request_timeout`且
retryable true，caller取消得到`request_aborted`且false。网络本身失败是`network_error`且true。

“retryable=true”不代表自动重试。一个POST超时后Server可能已经提交，调用方必须根据operation/report ID
查询权威状态，不能无条件再次发送。

finally会清timer和caller listener，避免长期React页面泄漏。测试应覆盖caller先、timeout先和fetch抛错。

## 5.6 有界读取

代码先检查合法数字`Content-Length`；若已超过预算，尝试cancel body并立即返回`response_too_large`。但不
信任声明值，仍通过ReadableStream逐chunk累计真实`byteLength`。超限后reader cancel失败也不能覆盖权威
size error。

所有chunk复制到精确长度Uint8Array，再用fatal UTF-8 decoder。非法字节不会变成replacement character后
继续JSON.parse。适合大文件或视频的API不应提高到无界，而应使用产品专用stream transport。

## 5.7 Content-Type与空响应

只接受`application/json`或任何以`+json`结尾的media type，允许`; charset=utf-8`等参数。HTML反向代理
错误页不能进入业务状态。

HEAD、204、205或`Content-Length: 0`返回`undefined`，所以logout guard应接受undefined。其他200空body会
在JSON parse阶段失败，这通常说明Server合同错误。

## 5.8 错误响应

非2xx若Content-Type不对、body过大、UTF-8/JSON错误或不匹配Error Envelope，会得到安全的
`invalid_error_response`或相应typed error，不回显raw body。若body和`X-Request-ID` header都有有效ID，
body优先；非法ID被忽略。

`Retry-After`支持整数秒和规范HTTP-date，最多24小时。它只填`retryAfterSeconds`，不会自动sleep/retry。

## 5.9 创建管理员client

```ts
import { createAdministratorApiClient } from "@sarmg/admin-web";

export const administratorApi = createAdministratorApiClient();
```

浏览器默认baseUrl是当前origin根。显式baseUrl仍必须同origin；Node环境没有location时必须传绝对HTTP(S)
URL。产品一般创建一个全应用单例，以便共享Session、mutation queue和generation。

`request(path, guard, init)`要求path留在`/api/v2/`。调用方不能预设X-CSRF-Token，也不能把credentials改成
非same-origin。若body存在且未指定content-type，会自动设置JSON。

## 5.10 为什么Session只在内存

权威登录状态由Server HttpOnly Cookie承载。Web可读取的Session对象和CSRF只保存在closure；刷新页面后
调用`GET /api/v2/auth/session`恢复。这样不会把token长期留在localStorage/sessionStorage/IndexedDB，
也不会引入跨tab同步和版本兼容状态。

不要照搬以下反模式：

```ts
// 错误：当前Foundation不允许。
sessionStorage.setItem("csrf", session.csrf_token);
```

## 5.11 Login、restore与logout

### Login

1. 共享 guard 先检查候选 username/password 结构；
2. generation递增，旧UI Session清空；
3. mutation进入全局Promise queue；
4. POST login，不携带旧CSRF；
5. strict Session guard；
6. 更新仅供后续排队logout的private transport snapshot；
7. 只有generation仍当前才发布给UI。

### Restore

并发调用共享一个Promise，并等待之前的login/logout mutation完成。成功响应只在generation未变化时发布。
401表示anonymous；Session shape/content/json/size错误会清理当前状态；普通网络错误留给UI error phase。

### Logout

调用时立即终止本地授权并递增generation，随后排队POST logout。若紧接刚完成但已被UI supersede的login，
private transport snapshot让logout仍能携带正确CSRF撤销Cookie。无论请求成功失败，finally都会清理snapshot。

## 5.12 认证竞态实例

### 旧401不应登出新Session

```text
请求A以Session 1发出
用户重新登录得到Session 2
请求A延迟返回401
```

请求A保存了dispatch generation和Session object。回调发现它们已不是current，就不invalidate Session 2。

### Restore不能覆盖Logout

```text
restore发出
用户点击logout，generation递增并本地anonymous
restore稍后成功返回
```

restore的`requireCurrentOperation`失败，不发布旧Session。

### 连续Login按调用顺序处理Cookie

两个login网络mutation串行，避免浏览器Set-Cookie完成顺序与调用顺序相反。较早login即使成功，其generation
已非current，会返回superseded而不发布。

## 5.13 React Hook

```tsx
import { useAdministratorSession } from "@sarmg/admin-web/react";
import { administratorApi } from "./api";

export function App() {
  const auth = useAdministratorSession(administratorApi);
  if (auth.phase === "loading") return <p>正在检查登录状态…</p>;
  if (auth.phase === "anonymous") return <Login onLogin={auth.login} />;
  if (auth.phase === "error") return <Recovery error={auth.error} />;
  return <Console session={auth.session} onLogout={auth.logout} />;
}
```

四态union保证每个phase对应合法session/error组合。hook mount自动restore；active client ref和自己的state
generation阻止组件卸载、client替换或旧Promise更新当前state。login失败会恢复client当前状态并把错误继续
抛给表单处理。

## 5.14 Vite与工具链

```ts
import { createSarmgReactViteConfig } from "@sarmg/admin-web/vite";

export default createSarmgReactViteConfig();
```

helper启用React plugin，输出`dist`并每次清空。产品可传`base`，其余业务build设置若确有需求应在不破坏
baseline的前提下审查。`assertAdministratorWebToolchain`检查package manifest和`.node-version`中的精确
Node/React/Vite/TypeScript版本，不接受范围。

## 5.15 成功响应guard

共享client只知道管理员Session和transport错误，不知道产品的Host、媒体、摄像头或Sunshine DTO。每个
业务调用必须传真实guard：

```ts
function isHealth(value: unknown): value is { status: "ok" } {
  return typeof value === "object" && value !== null &&
    Object.keys(value).length === 1 &&
    (value as Record<string, unknown>).status === "ok";
}

const health = await administratorApi.request("/api/v2/health", isHealth);
```

实际项目应避免上述示例中的重复type assertion，可写通用`isRecord/hasExactKeys`产品helper。关键是不能传
`() => true`。

## 5.16 本章练习

1. 分别构造cross-origin、redirect override和userinfo URL负例。
2. 模拟Content-Length较小但chunked实际超限，说明哪一层发现。
3. 画出“请求A旧401、Login B成功”的generation变化。
4. 解释为何logout先本地anonymous但仍必须等待Server mutation。
5. 为一个产品health response写exact runtime guard。
6. 找出为什么用sessionStorage存CSRF会扩大安全与兼容状态面。
