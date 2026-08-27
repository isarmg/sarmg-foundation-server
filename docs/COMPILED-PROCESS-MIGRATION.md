# 编译期进程模块迁移门禁

目标契约由 `upstream/REQUIREMENTS-AND-BOUNDARIES.md` 定义。本文件只记录 platform 层必须
提供的机制和当前差距。

## 目标

- manifest 声明模块 worker binary、固定 loopback/Unix-socket binding、gateway path、健康路径、
  数据所有权和 capability；不再声明管理员可配置的公共 URL。
- Union Cargo feature 决定编译哪些 manifest、gateway adapter、前端入口和 supervisor unit。
- `sarmg-platform-core` 只提供描述和验证，不能导入模块 DTO。
- `sarmg-platform-axum` 只提供受限内部代理/身份/错误映射，不合并模块业务 Router。
- `sarmg-platform-postgres` 只提供连接、migration/readiness；schema/role/SQL 继续由模块拥有。

## 当前差距

| 项目 | 当前状态 | 切换门禁 |
|---|---|---|
| Sunshine | Union 进程、Union SQLite/AppState | 导出/回滚、独立 schema/role、worker 与内部身份 |
| 主机监控 | Union 进程、Union SQLite/AppState | Agent 路由兼容、历史导入、worker 与内部身份 |
| Sentinel | 独立公网服务模型 | loopback、静态 gateway、Union supervisor、取消独立发布 |
| Photo Backup | 独立公网服务模型 | loopback、移动 API 网关、大 body/Range 测试、取消独立发布 |
| Dufs | 独立公网服务模型 | loopback、前缀/网关适配、大文件流测试；SQLite 保留 |

## 禁止的捷径

- 不在 Sunshine/主机仍持有同一个 SQLite 独占锁时启动第二进程。
- 不用空壳 worker 或健康探针替代真实业务拆分。
- 不把 `SARMG_*_URL` 从用户环境换成另一个可编辑配置字段；最终 binding 必须来自已编译清单。
- 不把 Union Cookie 原样转发给 worker，不信任所有 loopback 请求。
- 不因拆进程而把数据库合并进 `public` schema 或增加跨模块 SQL。

完成条件与官方 profile 见 `upstream/BUILD-AND-MODULE-ARCHITECTURE.md`。
