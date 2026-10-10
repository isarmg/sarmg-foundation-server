# ADR-0008：产品差异不得通过产品名称分支表达

- 状态：已采纳
- 日期：2026-09-02

## 后果

内部模块和能力应按技术职责命名，如 `admin_axum`、`admin_hyper`、`sqlite`、`linux-openat2`，不以产品名字选择不同实现。
`scripts/check-xcss.py` 检查 Rust 各依赖作用域、工作区、target、补丁和别名，
以及唯一根 npm 包的依赖声明。Rust 能力使用同一 crate 的内部模块，Web 能力使用同一
`@xcss/web` 包的公开子路径；没有独立内部包、工作区版本或内部路径依赖。外部本地路径不能成为正式依赖。
