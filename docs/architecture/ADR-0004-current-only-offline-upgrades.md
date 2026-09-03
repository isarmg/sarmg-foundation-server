# ADR-0004：current-only 与离线升级所有权

- 状态：Accepted
- 日期：2026-09-02

## 决策

Foundation 和在线产品只定义、创建、读取唯一当前格式。历史 parser、升级图、备份、恢复 journal 和格式
转换全部由 `sarmg-upgrade` 离线拥有。

## 后果

在线代码不得出现 legacy reader、兼容模式、双读写或隐式 fallback。任何持久格式变更必须先有脱敏 source
fixture、精确升级边和可回滚备份。
