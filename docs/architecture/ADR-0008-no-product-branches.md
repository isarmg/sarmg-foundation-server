# ADR-0008：产品差异不得通过产品名称分支表达

- 状态：Accepted
- 日期：2026-09-02

## 决策

Foundation 源码、Feature、Profile 和 Capability 中不得按产品 ID 分支或使用产品名称。允许的差异只有
Profile、Capability、Adapter/Trait、产品业务 Schema，以及 `sarmg-upgrade` 中的历史格式 Adapter。

## 后果

技术 Feature 应命名为 `axum`、`hyper`、`sqlite`、`linux-openat2`、`mobile-ffi` 等。合规工具扫描产品名
Feature 和 Foundation 对产品 crate 的反向依赖。
