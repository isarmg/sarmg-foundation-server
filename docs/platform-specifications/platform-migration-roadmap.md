# 平台化迁移路线

## 状态

本文件落实《Sarmg Foundation 上游平台化改造实施手册》的治理入口。Foundation `0.5.0` 是冻结基线，
提供 P5–P12 的当前平台实现，但不是 Foundation 1.0 完成声明；1.0 仍以全部消费者采用且无例外为门槛。

## 永久依赖方向

```text
第三方依赖 -> sarmg-foundation-server -> 产品 Adapter -> 产品业务
历史状态 ---------------------------------------> sarmg-upgrade
```

Foundation 不得依赖产品 crate，不按 `product_id` 分支，不提供产品名 Feature。运行形态只能通过正式 Profile
表达；业务差异只能通过 Adapter/Trait 表达；历史格式只能由离线升级边表达。

## 纵向切片完成条件

每项能力依次完成：平台提案、产品无关测试、唯一当前合同、Foundation 实现、一个参考产品 Adapter、删除
参考产品旧实现、不可变 revision 独立检出验证、扩展到其他消费者、禁止回退门禁。不得用空 crate 或未被
真实消费者采用的计划 API 代替完成证据。

## 当前顺序

1. P0：冻结 commit、版本、Schema fingerprint、脱敏 fixture 和行为 Golden Test。
2. P1：上游平台定位、ADR、Profile、Capability、临时例外。
3. P2：产品清单、合规工具、自动消费者矩阵、Testkit。
4. P3–P4：状态文件、平台数据库、Schema Composer、管理员控制面；Sunshine 为首个参考消费者。
5. P5–P12：服务端 Runtime/Web、文件安全、网络/加密、操作状态机已进入 Server Foundation；Client、Mobile FFI 归独立 Client Foundation；产品迁移按消费者矩阵推进。
6. P13：全部产品精确锁定同一不可变 release、独立构建且无临时例外后发布 1.0。

持久格式发生变化前必须先发布对应的 `sarmg-upgrade` source fixture 和升级边。在线产品只读取唯一当前
格式，不携带 legacy reader、双读写或兼容 fallback。
