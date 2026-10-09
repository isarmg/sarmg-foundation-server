# ADR-0008：产品差异不得通过产品名称分支表达

- 状态：Accepted
- 日期：2026-09-02

## 后果

技术 Feature 应命名为 `axum`、`hyper`、`sqlite`、`linux-openat2` 等。
`scripts/check-xcss.py` 检查 Rust 各依赖作用域、workspace、target、patch 和别名，
以及根目录和各 Web 包的依赖声明。内部包使用本仓库路径或精确 workspace 版本，
其他 Xcss 包和外部本地路径不能成为 xcss 的依赖。
