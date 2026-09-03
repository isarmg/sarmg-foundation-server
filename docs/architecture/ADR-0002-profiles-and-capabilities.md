# ADR-0002：Profile 与 Capability 模型

- 状态：Accepted
- 日期：2026-09-02

## 决策

产品组件必须选择 Foundation 发布的有限 Profile，并只能声明该 Profile 允许的 Capability。Profile 固定
安全常量、正式 target 和适配边界；产品不得自由拼装或覆盖这些政策。

## 后果

通用运行形态差异通过 Profile 表达。新增 Profile/Capability 必须有产品无关名称、规范、测试和消费者迁移
证据，不能只是某个产品的别名。
