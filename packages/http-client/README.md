# @sarmg/http-client

`@sarmg/http-client@0.3.0` 提供一个公开请求函数 `requestJson<T>` 和结构化错误 `ApiClientError`。它统一
same-origin URL、cookie credential、unsafe method CSRF、超时/调用方取消、响应字节预算、严格 JSON
Content-Type、Error Envelope 与 `Retry-After` 解析。

```ts
import { requestJson } from "@sarmg/http-client";

const candidate: unknown = await requestJson("/api/v2/health", {
  baseUrl: window.location.origin,
});
// 在这里使用产品自己的 runtime guard 验证 candidate。
```

默认 timeout 是 10 秒；允许范围 1～120000 ms。成功响应默认上限 2 MiB、硬上限 64 MiB，错误正文固定
上限 64 KiB；代码先检查 `Content-Length`，再流式累计真实 bytes，超限会尝试取消 body。只接受
`application/json` 或 `+json`，UTF-8 使用 fatal decode。URL 仅允许 HTTP(S)、无 userinfo、与 base origin
精确一致，redirect 固定为 `error`。

传入 `csrfToken` 时只向 unsafe method 注入；本包不读取任何存储，也不持有 Session。401 callback 会被
await，但其异常不会覆盖服务返回的权威错误。请求从不自动重试，因为通用层无法判断 mutation 是否幂等。
泛型 `T` 只改善编译体验，不验证成功响应。

本包把 `@sarmg/contracts@0.3.0` 声明为精确 peer dependency；消费者必须同时锁定两者。它不支持跨
origin、文件流、无界响应、自动 retry、业务 DTO guard、认证状态机或旧 Error Envelope。
