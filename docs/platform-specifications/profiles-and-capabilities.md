# Profile 与 Capability 规范

Profile 是 Foundation 发布并验证过的有限能力组合。产品的 `sarmg-product.toml` 必须为每个组件选择一个
Profile，并声明该 Profile 的全部必选 Capability；只能追加 Profile 明确允许的可选 Capability。

第一代 Profile 是：`server-control-plane`、`server-filesystem`、`desktop-agent`、`mobile-agent`、
`offline-tool`、`web-react-admin`、`web-embedded-native`。机器事实源位于 [`profiles/`](../../profiles)。

产品清单不得声明 Argon2 参数、Session 超时、Cookie 属性、管理员表名、CSRF Header、Server Rust 版本、
React/Vite 版本或正式 Server target。这些只能由 Profile/平台政策决定。产品差异不得进入 Profile 名称；
无法用通用运行形态表达的差异属于产品 Adapter 或业务 Schema。
