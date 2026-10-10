# ADR-0005：平台表与产品表共用一个 SQLite

- 状态：已采纳
- 日期：2026-09-02

## 决策

`server-control-plane` 的平台表与产品业务表组合进同一 SQLite Schema。xcss 独占 `_common_*` 名称，
产品维护 `schema/product.sql`，结构组合工具生成完整当前结构和结构指纹。

## 后果

跨平台/业务操作可以保持事务一致性；产品不得修改平台 DDL 或创建 `_common_*` 对象。平台 DDL 改动也是
持久格式改动，必须同步当前 Schema、结构指纹和正反验证；不匹配的状态在启动时拒绝。
