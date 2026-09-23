# 平台能力与接入边界

Foundation 的当前版本见根 README；包、Profile 和 Capability 定义共同描述发布接口。
消费者是否通过验收由清单、锁文件、独立构建和消费者矩阵决定，不能由上游测试结果代替。

## 依赖方向

```text
第三方库 -> sarmg-foundation-server -> 产品 Adapter -> 产品业务
当前状态离线维护 ---------------------------> sarmg-upgrade
```

Foundation 不依赖产品 crate，不按 `product_id` 分支，不提供产品名 Feature。
运行形态通过 Profile 表达，公共差异通过 Capability 和 Adapter/Trait 表达；
产品数据结构、业务协议和业务生命周期由消费者拥有。
`scripts/check-foundation.py` 检查 Rust/npm 内部依赖身份、各声明作用域和仓库外本地路径。

## 公共能力

- Rust：管理员认证、当前 Schema 身份、SQLite、状态文件、运行时生命周期、文件系统安全、秘密封装、有限 HTTP 请求及操作状态机。
- Web：管理员客户端、React Shell、原生 ESM 适配、UI、设计令牌、字体和构建工具链。
- 工具：Schema Composer、源码与发行一致性检查、产品无关 Testkit 和消费者报告。
- Client 和 Mobile FFI 由独立的 Foundation Client 定义，Server Foundation 不导入这些实现。

## 接入验收

每项公共能力需要明确规范、当前合同、产品无关测试、Foundation 实现和实际消费者 Adapter。
消费者固定完整不可变 revision 或发行包，并独立构建和运行其业务测试；未完成接入的能力应如实记录状态。
产品业务验证保留在产品仓库，Foundation 不以读取下游源码或私有 fixture 作为构建前提。

## 状态维护

在线产品只定义、创建和读取唯一当前格式。持久格式变化时同步更新当前 Schema、fingerprint、fixture 和验证；
不匹配的状态在启动时拒绝，不携带历史 reader、双读写或兼容 fallback。
`sarmg-upgrade` 负责服务端当前状态的离线维护，不要求实现历史版本转换。
Foundation 不保存指向其私有目录的 baseline，也不根据下游 fixture 布局决定构建或发布结果。
