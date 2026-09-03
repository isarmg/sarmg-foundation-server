# ADR-0007：Foundation 单版本发布策略

- 状态：Accepted
- 日期：2026-09-02

## 决策

所有 Foundation Rust crate、npm package、Profile、Schema、工具和测试以一个 Foundation 版本发布。正式
消费者同时固定精确版本与完整 40 位 Git revision；Web 使用对应 release 的不可变、已校验制品。

## 后果

平台合同不会形成无法验证的组件版本组合。联调期 path/file 依赖不得进入稳定分支，消费者必须通过无 sibling
仓库的独立检出测试。
