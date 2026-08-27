# 平台与私有进程架构

## 唯一运行模型

```text
Browser / mobile / agent
          │ HTTPS
          ▼
Union public gateway + authentication + supervisor
          │ fixed loopback + gateway-v1
          ├── sentinel-monitor   127.0.0.1:18101
          ├── photo-backup      127.0.0.1:18102
          ├── dufs              127.0.0.1:18103
          ├── sunshine          127.0.0.1:18104
          └── host-monitoring   127.0.0.1:18105
```

所有业务模块的 `execution` 都是 `private_process`，`service` binding 必填。平台不再定义
进程内业务 Router、管理员可编辑的服务 URL 或 Rust 动态插件。`.so`/`.dll` ABI、共享
Axum state 和跨模块全局变量均不属于受支持边界。

## 编译期选择

Cargo feature 同时决定：

1. 哪份 manifest 进入 Union 静态 catalog；
2. 哪条固定网关 adapter 被编译；
3. 哪个 worker 被 `union-builder` 编译并安装到 `libexec/union/modules/<id>`；
4. 哪个前端入口进入同一静态资源发行物；
5. supervisor 是否创建对应进程。

运行时复制一个额外二进制不会增加模块；未编译 feature 时不存在 catalog 项、路由、导航、
健康任务或进程任务。运行配置只能提供数据库凭据、业务秘密和存储目录，不能覆盖 `binary`、
`bind` 或 `gateway_prefix`。

`ui.kind=console` 表示前端视图在编译 Union Web 时静态链接，但业务请求仍转发到私有
worker；`ui.kind=gateway` 表示模块自带的前端资源由同一固定网关前缀提供。二者都不是
进程内后端，也不会引入运行时插件加载。

## 静态清单约束

`ModuleCatalog` 在路由开放前验证：

- module id、安装文件名、loopback socket、gateway prefix 全局唯一；
- 安装文件名必须等于 module id，网关必须精确为 `/modules/<id>`；
- liveness/readiness 是不同的规范绝对路径；
- capability 不重复且使用稳定的小写标识；
- PostgreSQL 环境变量、schema 和 role 合法且不被两个模块复用；
- SQLite state directory 是 Union 数据目录内的安全相对路径。

JSON Schema 负责形状与字段白名单，Rust 校验负责跨字段和跨模块唯一性。两者都必须通过。

## 内部身份与请求边界

Union 每次启动为每个 audience 生成独立的 64 个十六进制字符 token，并通过私有启动环境传给
worker。每个请求（含健康检查）都必须携带：

```text
X-Union-Module-Protocol: gateway-v1
X-Union-Module-Audience: <module-id>
X-Union-Module-Token: <process-scoped token>
X-Forwarded-Prefix: /modules/<module-id>
```

worker 恒定时间校验完整四元组；健康响应只回显 protocol 和 audience，不回显 token。
Union 在探测确认协议前不开放代理。Union 必须覆盖客户端提供的内部头和 forwarded 头，删除
hop-by-hop 头以及自己的 session/CSRF Cookie。模块用户凭据只有在该模块协议明确需要时才
转发，不能把 Union 长效会话当成 worker 身份。

loopback 是网络收敛，不代替内部认证。同一 Unix 服务账户仍属于共同信任域；若将来需要抵御
恶意 sibling worker，应增加独立 UID、受限 `/proc` 和 Unix socket 文件权限，而不是扩大
HTTP token 权限。

## 生命周期与健康

supervisor 使用固定路径启动、捕获 PID、优雅发送 SIGTERM，并在宽限期后强制结束。异常退出
采用有上限的指数退避；每个新进程代重新经过 liveness、readiness 和 gateway 协议门禁。

- liveness：进程仍能处理内部 HTTP。
- readiness：本模块数据库、文件系统及关键依赖可服务。
- gateway compatibility：精确的 protocol/audience/prefix/token 已由 worker 证明。

三者不能合并成一个缓存布尔值。Union 停机时先停止接收并排空代理响应，再关闭 worker，
避免在上传或下载中途切断后端。

## 发布边界

模块可在各自源码仓库独立测试和回滚，但不发布独立程序、容器或 Release。生产回滚以完整
Union release manifest 为单位。module id、gateway、database ownership 或内部身份变化均是
平台契约的破坏性变更。
