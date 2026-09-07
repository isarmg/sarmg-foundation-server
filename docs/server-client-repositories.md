# Server / Client 独立仓库约定

产品服务端目录及仓库使用 `-server` 后缀，已有独立安装客户端使用 `-client` 后缀。
管理 Server 的 Web 随 Server；管理客户端自身的 Web 随 Client。没有独立安装客户端的产品不建立空 Client 仓库。

Foundation Server 与 Foundation Client 相互独立。Server 不依赖 `sarmg-client-*` 或移动 FFI；
跨端产品协议留在对应产品的 Server 仓库作为唯一源码，Client 固定完整 Git revision 与精确版本，
不得通过相邻工作区路径或浮动分支依赖另一端。协议 crate 不依赖 Server 可执行程序。

`consumers/repositories.toml` 只记录 Server 消费者及离线维护工具。Client SDK 基线和测试由
`sarmg-foundation-client` 维护。产品历史与发布标签不重写，旧发布记录不作为新拆分提交的验收证据。
`sarmg-upgrade` 不因仓库改名自动增加版本或客户端状态支持；第三方字体仓库不按产品端拆分。
