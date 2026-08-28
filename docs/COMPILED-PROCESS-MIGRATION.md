# 从编译期模块到发行内运行期插件

本文件保留原路径以便旧链接可达，但目标模型已经改变：Cargo feature 不再选择业务代码，Core
不编译业务模块。Builder 在发行构建阶段选择完整模块包；Runtime 在当前发行范围内发现和管理。

## 包迁移

每个模块产出独立 bundle：

```text
manifest.json
backend/
frontend/
permissions/
config/
migrations/       # embedded migration 可无 SQL 文件
version/
```

迁移完成必须证明：manifest/schema/Rust validator 通过；executable 与配置路径存在且不可逃逸；
frontend identity/components 匹配；route rewrite 与 auth 边界符合真实 worker；permission、migration、
service 和 event 声明可注册；liveness/readiness、启动、停机和失败回退可演练。

## 运行时切换

1. Builder 生成带内容摘要的发行目录，不修改当前 active slot。
2. Runtime 在 staging 中解析全部 bundled manifest，完成 compatibility 与依赖拓扑。
3. 配置先过模块 JSON Schema；敏感字段不写日志或命令行。
4. 数据 migration 前创建模块所有权范围的备份/恢复点。
5. 启动候选模块并验证 health、gateway identity、关键 API 与 Web entry。
6. 原子切换 active catalog；失败恢复旧 slot，不能把 schema rollback 等同文件 rollback。

运行时启停当前发行已有模块不重建 Core/Web；增加或删除发行包含的模块仍由 Builder 生成新发行，
不支持从 URL、OCI registry 或插件市场在线引入任意代码。

## 认证迁移

Web 管理路由优先 `auth=platform`。Agent、设备、移动 API key、媒体 token 等领域协议可
`auth=module`，但必须继续经过 Core Gateway。Dufs 自有 ACL 暂为明确过渡例外，不能扩散成新模块
默认方案。

## 数据切换

Sunshine/主机监控旧 SQLite 数据迁移须冻结旧写入、导入模块自有 PostgreSQL schema、记录来源
fingerprint/行数/摘要、复验并保留 rollback journal。Sentinel/Photo 保持各自 PostgreSQL 所有权。
Dufs 保持与文件系统同故障域的 embedded SQLite migration。

## 生产门禁

- unknown field/path traversal/权限引用/route rewrite/dependency cycle 任一失败即不激活。
- process 拒绝公网 bind，内部 token/audience/prefix 不匹配即不代理。
- platform-auth 写请求必须经过 RBAC/CSRF；module-auth 不得成为旁路公网入口。
- frontend 禁止远程 origin 和第二份 React；卸载后不残留 route/menu/style。
- migration、安装、升级、进程故障、优雅停机、发行回滚和数据恢复都需真实目录测试。

验收报告必须区分“构建成功”“契约成功”“运行时激活成功”“功能成功”“数据切换成功”。
