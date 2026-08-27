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

平台核心不得导入任何模块的 DTO、数据库表或业务状态。只有最终发行程序知道安装了哪些
模块。进程内模块通过编译期注册组装，独立服务模块通过 manifest 和受限适配器注册。

## 模块类型

- `in_process`：首批只用于 Sunshine 和主机监控。模块贡献 Axum console/public Router、
  前端视图、后台任务、健康状态和自己的 migration。
- `service`：用于 Sentinel、Photo Backup 和 Dufs。平台保存非秘密 base URL，读取公开
  liveness，展示导航；不转发平台 Cookie，也不直接读取服务数据库。

不实现 Rust `.so`/`.dll` 动态插件。Rust ABI、Axum/Tower 类型和共享状态版本不稳定，动态
插件会把升级边界变成不可审计的运行时失败。需要独立升级的模块使用进程边界。

## 身份边界

首版外部模块保留自己的登录。以后统一登录时使用短时、有 audience 的签名票据或 OIDC；
禁止共享用户表、session 表、Cookie 密钥和平台 Bearer token。反向代理接入必须单独进行
威胁建模，不能把管理员提供的 URL 变成通用开放代理。

## 发布边界

`platform-core` 遵循语义版本。模块 manifest 是公共 API；删除 capability、修改模块 id、
改变数据库所有权或收紧身份契约均视为破坏性变更。每个消费者仍可独立构建、发布和回滚。

