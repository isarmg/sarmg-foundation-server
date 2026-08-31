# 03. Rust 错误与 SQLite 基础

## 3.1 ErrorCode

`ErrorCode` 最长 128 bytes，以小写 ASCII 字母开始，只含小写字母、数字、点、下划线和连字符。它是机器
分支的稳定原语，不是展示文案，也不自动分配产品 namespace。

## 3.2 ErrorEnvelope

envelope 包含 code、用户可理解 message、可选 request ID、retryable 和对象型 details。message/details
不得包含 Secret、SQL、内部路径或未清理上游正文；服务日志另存受限诊断。

## 3.3 HTTP status

共享类型只提供明确常用状态映射和默认 retry 语义。产品仍决定哪个业务错误使用哪个 code/status，不能
因为有通用 envelope 就把所有错误变成 500。

## 3.4 SQLite baseline

`open_pool` 设置 WAL、foreign keys、5 秒 busy timeout、`synchronous=FULL`、10 秒 acquire timeout 和正
连接数。它提供 integrity、foreign key check 与会报告 busy 的 TRUNCATE checkpoint。

## 3.5 它不证明什么

连接成功不证明：路径未被 symlink 替换、文件 owner/mode 安全、数据库属于当前产品、Schema 正确、只有
一个实例、允许创建新库、备份一致或 migration 安全。这些必须由消费者在外层完成。

## 3.6 WAL

WAL 允许读写并行，但已提交页可能仍在 `-wal`。复制主文件不是备份。checkpoint busy 是真实状态，不能
吞掉错误并声称备份完成。

## 3.7 连接数

更多连接不等于更多 SQLite 写吞吐。产品通常仍需单 writer、短事务或其他明确并发架构。Foundation 只
验证连接数为正并应用 baseline。

## 3.8 消费者集成清单

路径锚定 -> owner/mode/type -> instance/maintenance lock -> 当前 metadata/DDL -> 是否允许初始化 ->
`open_pool` -> 产品事务/doctor。任何顺序变化需在消费者 threat model 中证明。

## 3.9 测试重点

ErrorCode 边界、envelope serialization、status/retry；SQLite pragma、integrity/FK violations、checkpoint
busy、acquire timeout 和 invalid pool size。产品 Schema 不应进入 Foundation fixture。
