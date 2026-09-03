# ADR-0001：Foundation 是构建期中央平台

- 状态：Accepted
- 日期：2026-09-02

## 决策

Foundation 是所有 Sarmg 产品的平台规范、公共实现、工具链和一致性事实源。产品在构建时锁定并带入
Foundation；生产中仍独立运行，不连接中央 Foundation 服务。

## 后果

平台责任即使暂时只有一个参考消费者也应在 Foundation 纵向落地。Foundation 不拥有产品业务模型、业务
数据或外部业务协议，也不形成新的生产故障域。
