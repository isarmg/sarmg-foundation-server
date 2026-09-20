# Server / Client 独立仓库约定

产品服务端目录及仓库使用 `-server` 后缀，已有独立安装客户端使用 `-client` 后缀。
管理 Server 的 Web 随 Server；管理客户端自身的 Web 随 Client。没有独立安装客户端的产品不建立空 Client 仓库。

Foundation Server 与 Foundation Client 相互独立。Server 不依赖 `sarmg-client-*` 或移动 FFI；
跨端产品协议留在对应产品的 Server 仓库作为唯一源码，Client 固定完整 Git revision 与精确版本，
不得通过相邻工作区路径或浮动分支依赖另一端。协议 crate 不依赖 Server 可执行程序。

`consumers/repositories.toml` 只记录 Server 消费者及离线维护工具。Client SDK 基线和测试由
`sarmg-foundation-client` 维护。产品历史与发布标签不重写，旧发布记录不作为新拆分提交的验收证据。
`sarmg-upgrade` 不因仓库改名自动增加版本或客户端状态支持；第三方字体仓库不按产品端拆分。

## 职责判定

| 所属 | 应负责 | 不应负责 |
|---|---|---|
| Foundation Server | Server 进程、管理 Server 的 Web、管理员认证、服务端 HTTP/数据库/文件原语、通用管理 UI、离线服务端工具 | Client 本机状态、移动 FFI、产品业务 DTO、产品配对协议与产品页面 |
| Foundation Client | 桌面/移动 Client 运行时、Spool、原生终端和服务生命周期、本机只读状态、客户端文件/秘密安全、有界输入解析、移动 FFI | 管理 Server 的 Web、产品协议错误码、产品恢复文案和第三方本机服务策略 |
| 产品 | 端点与 wire、实例/设备/硬件状态机、业务错误目录、业务页面、更严格的产品安全规则和产品发布验收 | 复制 Foundation 已发布实现形成第二事实源 |

进入任一 Foundation 的能力必须产品中立、能在 Foundation 内独立测试、允许产品继续加强约束，并有明确的跨产品复用场景。单产品能力先留在产品；不能因为多个产品都叫“配对”就把不同 wire、状态码和恢复流程合并成公共协议。

`@sarmg/admin-ui` 的内容块只提供布局、色板和无障碍展示原语，因此属于 Foundation Server。实例统计、授权码、CPU/GPU/SSD/RAM、摄像头、Sunshine 控制等内容与行为属于产品。消费者必须从锁定的发布包导入样式，不在产品仓库保存 Foundation CSS 快照。

`sarmg-secure-xml` 只实现产品中立的输入、深度、节点、文本和时间预算，实际消费者是 Client，因此由 Foundation Client 提供。Foundation Server 不为 Client 保留镜像包；ONVIF SOAP、命名空间、字段和预算值仍属于 Sentinel。
