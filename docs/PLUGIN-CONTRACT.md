# Plugin Manifest、Platform SDK 与 Event API

## Manifest v1

事实源为 `contracts/plugin-manifest-v1.schema.json` 与 `sarmg-platform-core::PluginManifest`。
JSON Schema 约束字段形状，Rust 负责跨字段、跨模块和安全语义。仅通过其中一个不算可激活。

这里的“契约事实源”指格式和语义；某个模块的具体字段只以该模块仓库的 `manifest.json` 为准。
Platform 的 `tests/fixtures` 是中性测试数据，不是业务清单镜像。Builder 必须从选定模块 revision
直接读取清单并完成双重校验，不能回退到 Platform 内的副本。

顶层必填：

```text
manifest_version / id / display_name / description
version / version_metadata / compatibility / dependencies
execution / backend / frontend / permissions / migrations / configuration
health / lifecycle / services / events
```

执行模式：

- `in_process`：bundle 内 WASI Component artifact + callback health；原生 Rust `.so`/不稳定 ABI 禁止。
- `process`：`backend/` 下安全相对 executable、非 shell args/config env mapping、loopback bind。
- `container`：image 必须配精确 `sha256:` digest，HTTP health。
- `service`：只引用 Manifest 中声明的 service discovery name，不保存任意 endpoint URL。

依赖版本不匹配、required dependency 缺失、自依赖、循环依赖、重复 service、权限越界、route
capture 不同构、未知组件、menu 指向未声明 route、路径 traversal 或不合法 migration 形状均拒绝。

每条 backend route 的可选 `request_body` 声明 `max_bytes` 和 `total_timeout_seconds`。省略时使用
1 MiB/30 秒安全默认值；Core 必须在 Gateway 读取/转发时同时执行两项上限，worker 可以进一步
收紧但不能放宽。大文件模块应只给精确上传 route 较大的窗口，不应放宽整个模块 API。

PostgreSQL migration 必须有安全目录和 schema；SQLite migration 必须有目录；`embedded` 用于
编译在模块内部的迁移，必须不声明虚假目录/schema。

## Process contract

Runtime 总是注入以下保留变量，Manifest 不能覆盖：

```text
UNION_PLUGIN_ID
UNION_PLUGIN_VERSION
UNION_PLUGIN_BIND
UNION_PLUGIN_PORT
UNION_PLUGIN_PACKAGE_ROOT
UNION_PLUGIN_CONFIG
UNION_MODULE_PROTOCOL
UNION_MODULE_AUDIENCE
UNION_MODULE_TOKEN
UNION_MODULE_PREFIX
```

`UNION_PLUGIN_CONFIG` 指向 schema 已验证、只读、绝对路径 JSON 文件。SDK
`ProcessContext::from_env` 校验身份、loopback bind、端口、package/config 路径和 gateway token；
`load_configuration<T>` 只读取 regular、non-symlink、最大 1 MiB 的 JSON。

Manifest v2 不再桥接模块专属配置或 bind 环境变量。Worker 必须从 `UNION_PLUGIN_CONFIG` 指向的
已验证 JSON 读取配置，并只从 `UNION_PLUGIN_BIND` 读取 Core 分配的监听地址；Core 不按 module id
特判，也不向子进程注入旧名称别名。

内部 `UNION_MODULE_PREFIX` 是 `/api/modules/<id>`；前端资源前缀另为
`/modules/<id>/assets/`，二者不得混用。token 只用于 runtime-to-worker 身份，不是用户权限。

`auth=platform` 路由还必须携带唯一的 `X-Union-Principal`。该值是 1–128 字节的规范
UTF-8 用户名（无首尾空白或控制字符），由 Core 在完成会话认证与 RBAC 后覆盖写入；Worker
必须通过 `sarmg-platform-gateway::parse_principal` 读取，不能使用仅接受 ASCII 的
`HeaderValue::to_str`，也不能回退到本地 Cookie/Bearer 身份。`auth=module` 路由不得把该头当作
设备凭据，其能力令牌仍由模块自己的稳定协议定义。

## Platform SDK

`sarmg-platform-sdk` 暴露 configuration、authorization、audit、structured log、task、notification、
service discovery 和 event publisher 接口。模块只依赖这些稳定 trait，不引用 Core state、Axum
Router 或数据库内部实现。

WASI host adapter 通过 `InProcessHttpHandler` 统一分派：Core 先按 Manifest 选择 route id、完成认证与
header 过滤，再传入 method/path/query/bounded body/Actor。大 body 和 streaming 不属于此 ABI。
process/container/service 使用等价 wire handshake，API 版本由 `PluginHandshake` 显式携带。

## Event API

`sarmg-platform-events` 的 Envelope 包含 id、topic、整数 schema version、producer、UTC time、
correlation id 与 JSON payload。发布 topic 必须由生产模块命名空间拥有；整数 schema version
按 `<version>.0.0` 与订阅 SemVer range 比较，handler 返回 acknowledge/retry/reject。
`at_least_once` 消费者必须
按 event id 幂等，事件不能用来构造跨模块同步调用链或分布式事务幻觉。

## Frontend SDK

Shell 提供 `hostSdk.react`、认证状态、权限查询、same-origin API client 与受控导航。入口对象的
moduleId/version/pluginApiVersion 必须与 Manifest 相同；styles 和 entry 只能从同一已验证 bundle
加载。卸载时 Shell 必须撤销 routes/menu/styles 和模块注册的清理函数。
