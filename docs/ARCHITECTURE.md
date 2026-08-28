# 混合模块架构

## 总体结构

```text
Builder-selected release contents
  ├── Core Platform          identity/RBAC/config/audit/tasks/notifications
  ├── Web Shell              layout/session/navigation/dynamic loader
  ├── Plugin Runtime         discovery/validation/migration/lifecycle/gateway
  ├── Platform SDK           stable framework-neutral capabilities
  ├── Event Bus              versioned asynchronous integration
  └── Modules
       ├── in-process        low cost, low risk, frequent Core interaction
       └── process/container/service
                            heavy jobs, scaling or failure isolation
```

Core 不包含业务判断。Builder 可组合不同发行内容，但不会把业务源码编译进 Core。运行时 discovery
root 只能指向当前发行的只读 modules 目录，Manifest 的 `distribution` 必须为 `bundled`。staging、
健康检查和原子切换可用于新发行升级，不等于允许从网络安装未知模块。

模块仓库拥有自身 `manifest.json`，并且是该清单的唯一事实源。Platform 只发布 schema、Rust
validator、SDK 和通用测试夹具，不复制具体业务清单。Builder 从所选模块的源码 revision 直接读取
并校验清单，再把校验通过的原件装入发行包；因此清单变更不需要同步修改 Platform 仓库。

## 生命周期顺序

Runtime 对当前发行目录执行：

1. fail-closed 读取 manifest，拒绝未知字段和不安全路径；
2. 校验 Core、Platform API、Plugin API compatibility；
3. 校验 required/optional dependency、实际版本并做确定性拓扑排序；
4. 注册 permission definition 和 configuration schema；
5. 按模块所有权执行 migration；
6. 注册 service discovery、event publish/subscribe、backend route 与 frontend asset；
7. 按 execution mode 启动并完成 liveness/readiness；
8. 只有全部门禁通过后进入 active；停用按反向依赖顺序执行。

任何一步失败都不能留下半注册路由、权限或 migration 状态。升级需要 staging、兼容校验、备份与
模块级回滚证据；删除包不等于回滚数据。

## Web Shell

Shell 只提供布局、认证状态、导航、RBAC 和模块加载器。Manifest 声明 ESM `entry`、CSS `styles`、
组件白名单、Route 与 Menu；资源由 Core 解析为 `/modules/<id>/assets/<relative>`，禁止外部 URL。

入口默认导出：

```js
{ pluginApiVersion, moduleId, version, activate(hostSdk) }
```

`activate` 返回的 `components` 必须覆盖 Manifest 白名单。Shell 通过 `hostSdk.react` 提供唯一
React 实例；模块不得打包或运行时导入第二份 React/ReactDOM。安装、卸载和升级当前发行内模块
不要求重建 Web Shell。

## 认证和 Gateway

所有外部请求仍先经过 Core。canonical backend base 固定 `/api/modules/<id>`；route 的
`upstream_path` 完成 legacy worker 路由映射，capture 名和 wildcard 类型必须与 canonical path
一致，Core 不按 module id 硬编码业务 rewrite。

- `auth=platform`：Core session、RBAC 与写请求 CSRF 生效，permission 必填。
- `auth=module`：设备、Agent、移动端 API key 或领域凭证由 worker 校验，permission 必须为 null。

`auth=module` 不是绕过 Union 或允许 worker 公网监听。Dufs 当前保留自身 ACL 属于过渡边界；
其流量仍经 Core Gateway，后续只有在不破坏 WebDAV/Basic Auth 客户端时才迁移到统一身份。

## 数据边界

每个模块拥有独立 schema/database、migration 和 runtime role。禁止跨模块外键、共享可写表、
直接读取另一模块数据或从数据库 join 推导平台身份。跨模块集成使用稳定 Platform API、Plugin API
或 versioned event；禁止依赖内部 crate、内部表和循环同步调用链。

共享 PostgreSQL cluster 只是运维统一，不等于共享数据所有权。Dufs 的 SQLite 与文件系统提交
处于同一故障域，继续作为显式 `embedded` migration 例外。

## 演进边界

in-process 模块以 ABI-stable WASI Component 随发行打包并共享 Core 故障域，只适合有界 body、
低风险、低资源功能；不加载原生 Rust `.so`。大文件、媒体、长任务、
独立扩缩容或强隔离模块使用 process/container/service。SDK 的 in-process HTTP 接口故意使用
bounded bytes；需要 streaming 即是提升隔离级别的架构信号。
