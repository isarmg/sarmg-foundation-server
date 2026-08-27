# 平台与模块边界

## 依赖方向

```text
upstream contracts/design
          ↓
platform-core ← platform-axum / platform-postgres
          ↑
Sunshine / host-monitoring / service adapters
          ↑
Union distribution（唯一组装根）
```

平台核心不得导入任何模块的 DTO、数据库表或业务状态。只有最终发行程序知道编译了哪些
模块。新的唯一目标是编译期选择、运行时独立进程；旧的进程内 Router 与运行时 URL 注册只
作为有期限的迁移机制，见 `COMPILED-PROCESS-MIGRATION.md`。

## 模块类型（目标）

- `process`：五个业务模块都编译为 Union 私有 worker。manifest 提供固定 gateway path、私有
  binding 和健康路径；Union 编译 feature 决定是否包含适配器、前端入口和 worker。
- `core`：Union 认证、公共网关、supervisor、发行 manifest 和系统健康，不是可选业务模块。

不实现 Rust `.so`/`.dll` 动态插件。Rust ABI、Axum/Tower 类型和共享状态版本不稳定，动态
插件会把升级边界变成不可审计的运行时失败。需要独立升级的模块使用进程边界。

## 身份边界

首版外部模块保留自己的登录。以后统一登录时使用短时、有 audience 的签名票据或 OIDC；
禁止共享用户表、session 表、Cookie 密钥和平台 Bearer token。反向代理接入必须单独进行
威胁建模，不能把管理员提供的 URL 变成通用开放代理。

## 发布边界

`platform-core` 遵循语义版本。模块 manifest 是公共 API；删除 capability、修改模块 id、
改变数据库所有权或收紧身份契约均视为破坏性变更。模块可独立测试和回滚源码，但只能随
Union 发行版发布；生产回滚以完整 Union release manifest 为单位。
