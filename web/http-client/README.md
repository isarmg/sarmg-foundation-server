# @xcss/web/http-client

`@xcss/web/http-client@1.0.0` 提供一个公开请求函数 `requestJson<T>` 和结构化错误 `ApiClientError`。它统一
同源 URL、Cookie 凭据、非安全 HTTP 方法的 CSRF、超时与调用方取消、响应字节预算、严格的 JSON
`Content-Type`、统一错误响应结构及 `Retry-After` 解析。

```ts
import { requestJson } from "@xcss/web/http-client";

const candidate: unknown = await requestJson("/api/v1/health", {
  baseUrl: window.location.origin,
});
// 在这里使用产品自己的 runtime guard 验证 candidate。
```

默认超时为 10 秒，允许范围为 1～120000 ms。成功响应默认上限为 2 MiB、硬上限为 64 MiB，错误正文固定
上限为 64 KiB；代码先检查 `Content-Length`，再以流式读取累计真实字节，超限时尝试取消正文读取。只接受
`application/json` 或 `+json`；UTF-8 解码遇到非法字节时立即失败。URL 仅允许 HTTP(S)，不得携带用户信息，且必须与基础来源
精确一致；`redirect` 固定为 `error`。

传入 `csrfToken` 时，只向非安全 HTTP 方法注入令牌；本包不读取任何存储，也不持有会话。处理 401 时会等待回调
完成，但回调异常不会覆盖服务返回的权威错误。请求从不自动重试，因为通用层无法判断修改操作是否幂等。
泛型 `T` 只改善编译体验，不验证成功响应。

HTTP 客户端与合同模块均由同一个 `@xcss/web@1.0.0` 包提供，消费者固定该单包的发行来源和摘要。
它不支持跨源请求、文件流、无界响应、自动重试、业务 DTO 校验、认证状态机或旧错误响应结构。
