# ADR-0005：平台表与产品表共用一个 SQLite

- 状态：Accepted
- 日期：2026-09-02

## 决策

`server-control-plane` 的平台表与产品业务表组合进同一 SQLite Schema。Foundation 独占 `_sarmg_*` 名称，
产品维护 `schema/product.sql`，Schema Composer 生成完整 current Schema 和 fingerprint。

## 后果

跨平台/业务操作可以保持事务一致性；产品不得修改平台 DDL 或创建 `_sarmg_*` 对象。平台 DDL 改动也是
持久格式改动，必须有升级边。
