# ADR-0008：产品差异不得通过产品名称分支表达

- 状态：Accepted
- 日期：2026-09-02

## 决策

Foundation 源码、Feature、Profile 和 Capability 中不得按产品 ID 分支或使用产品名称。允许的差异只有
Profile、Capability、Adapter/Trait 和产品业务 Schema。当前状态离线维护的产品 Adapter 由 `sarmg-upgrade` 拥有。

## 后果

技术 Feature 应命名为 `axum`、`hyper`、`sqlite`、`linux-openat2` 等。
`scripts/check-foundation.py` 检查 Rust 各依赖作用域、workspace、target、patch 和别名，
以及根目录和各 Web 包的依赖声明。内部包使用本仓库路径或精确 workspace 版本，
其他 Sarmg 包和外部本地路径不能成为 Foundation Server 的依赖。
