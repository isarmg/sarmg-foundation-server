# 05. HTTP Client 请求生命周期

## 5.1 请求入口

`requestJson<T>` 默认同源 credential、10 秒 timeout、2 MiB 成功响应上限；调用方可在硬上限 64 MiB 内
调整。泛型 `T` 只描述期望，不验证业务响应。

## 5.2 Abort 与 timeout

客户端将 caller signal 和内部 timeout 合并，完成后释放 timer/listener。caller abort、timeout 和网络
错误需要不同诊断；都不能自动断言 unsafe 请求未执行。

## 5.3 CSRF

只有 unsafe method 才添加经过验证的 CSRF token。helper 不拥有 auth store，也不自动刷新 Session；
401 可调用消费者回调，但权威 ApiClientError 仍需保留。

## 5.4 有界读取

先验证 JSON Content-Type，再通过 streaming reader 累计实际 bytes；不能只信任 Content-Length。超过
预算立即取消并返回受限错误，防止无界 `response.json()` 占用内存。

## 5.5 错误解析

非成功响应尝试解析严格 ErrorEnvelope 和 `Retry-After`。无效/非 JSON 上游正文不会原样暴露；状态、
request ID 与安全摘要足够调用方判断。

## 5.6 成功验证

成功 JSON 解析后仍是调用方业务边界。使用 `@sarmg/contracts` guard 或产品自有 schema 验证，再写入
状态。`requestJson<MyType>` 不能使网络对象自动变成 `MyType`。

## 5.7 重试边界

helper 不实现通用重试。GET、幂等 mutation、429/503、Retry-After 与 unknown 副作用需要产品语义，通用
库不能安全猜测。

## 5.8 测试

覆盖 Content-Type 参数、chunked 超限、声明长度欺骗、空 body、非法 JSON、caller abort、timeout、401
回调抛错、Retry-After 日期/秒和 unsafe CSRF。

## 5.9 集成原则

消费者显式依赖 contracts/http-client，包装产品 base URL、auth 和 response guard；不要 fork helper 或
通过 re-export 隐藏依赖。
