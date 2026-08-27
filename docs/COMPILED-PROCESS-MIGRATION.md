# 编译期私有进程迁移与验收门禁

本文件定义从历史部署切换到最终运行模型时必须满足的证据。最终模型本身只有
`private_process`，不存在运行时 URL 注册或进程内业务模块的兼容模式。

## 每个模块的完成条件

- manifest 通过 JSON Schema 和 `ModuleCatalog`，并锁定 binary/bind/gateway/health/database。
- worker 拒绝非 loopback bind，拒绝缺失或错误的 `gateway-v1` 四元组。
- UI、API、Cookie Path、redirect 和静态资产全部尊重编译前缀。
- liveness 与 readiness 独立；健康响应回显协议与 audience，但不泄露 token。
- supervisor 能处理启动失败、快速崩溃、退避、正常 SIGTERM 和超时强杀。
- Union 不转发自己的 Cookie，不接受客户端伪造的内部或 forwarded 头。
- 大 body、Range、HEAD、SSE/流媒体和客户端中断经过真实网关测试。
- module crate/binary 均不可发布，仓库没有独立 Release 工作流。

## 数据切换门禁

Sunshine 和主机监控在启用其 worker feature 前，必须完成旧 Union SQLite 的冻结、导入、核验
和可演练回滚。切换时只允许一个事实源接受写入。Sentinel/Photo 继续使用各自 PostgreSQL，
但入口、cookie/prefix 和发布边界必须切到 Union。Dufs 不迁移数据库，只迁移启动和网关边界。

## 官方 profile 验证

至少维护以下 profile，而不是穷举全部 feature 组合：

| profile | 模块 | 必须覆盖 |
|---|---|---|
| `minimal` | 无业务模块 | 没有模块路由、进程和导航 |
| `storage` | Photo + Dufs | 上传、Range、恢复、SQLite/文件故障 |
| `monitoring` | Sentinel + 主机 | SSE/媒体/Agent、高频写入和 PG readiness |
| `full` | 五个模块 | 端口唯一、进程监督、统一入口、安装/升级/回滚 |

每个 profile 必须从精确 Git revision 的干净源码构建，输出 release manifest 和 SHA-256，随后
在解压后的真实发行目录执行安装、首次启动、升级、故障重启、优雅停机与回滚验证。

## 明确禁止

- 将旧 `SARMG_*_URL` 换名后继续作为可编辑 upstream。
- 用空壳 worker、仅健康检查或子进程包装器冒充业务拆分。
- 让 worker 连接 Union SQLite/AppState/session 或共享另一个模块 role。
- 未验证 prefix 就以正文替换方式临时修补绝对 URL。
- 因 loopback 而跳过 audience/token 校验。
- 单独发布任一 module binary、容器或 GitHub Release。

验收报告必须区分“编译成功”“契约测试成功”“真实发行目录端到端成功”和“数据切换成功”；
其中任意一项缺失，都不能宣称该 profile 已可生产切换。
