# 10. 源码阅读路线、练习与术语表

## 10.1 按学习阶段阅读

### 第一阶段：公共面

先读根`Cargo.toml`、`package.json`、各crate/package manifest和README。目标是列出组件、public export、精确
内部依赖与工具链，不进入实现细节。

### 第二阶段：核心primitive

依次读`admin-auth`、`error`、`schema-identity`、`sqlite`、`server-target`。每个function同时读紧邻tests，
记录输入、typed error和“不保证”。

### 第三阶段：跨语言和Web

比较`sarmg-contracts/src/lib.rs`与`packages/contracts/src/index.ts`、Schema和fixture；再读http-client、
admin-web index、React hook和Vite helper。重点追踪一个unknown JSON何时变为trusted。

### 第四阶段：发布与消费者

读`foundation_policy.py`、package artifact工具、release实现和tests，再读consumer matrix与各产品采用点。最后
读workflow，验证权限和命令是否与文档一致。

## 10.2 按问题找入口

| 问题 | 首选源码 | 继续阅读 |
|---|---|---|
| 管理员 username/密码 | `rust/crates/sarmg-admin-auth/src/lib.rs` | contracts auth Schema/fixture、产品 startup/login |
| Origin/Host/CSRF | admin-auth authority/header函数 | 每个Server framework adapter与integration test |
| Cookie/token | admin-auth token/cookie函数 | 产品Session persistence/Cookie flags |
| Error JSON | `sarmg-error` | contracts fixture、http-client responseError |
| AdministratorSession | `sarmg-contracts`与`@sarmg/contracts` | admin-web、产品router |
| State/Release/Backup | contracts Rust/TS/Schema | release builder、sarmg-upgrade |
| SQLite fingerprint | `sarmg-schema-identity` | golden fixture、产品DDL |
| SQLx连接/诊断 | `sarmg-sqlite` | 产品path/lock/lifecycle |
| AMD64 Server | `sarmg-server-target` | 产品build.rs/release/start |
| URL/timeout/body/error | `packages/http-client/src/index.ts` | package tests、产品API wrapper |
| auth竞态 | `packages/admin-web/src/index.ts` | admin-web tests、React hook |
| React状态 | `packages/admin-web/src/react.tsx` | 产品App/login page |
| React/Vite精确版本 | admin-web toolchain与vite.ts | 产品package/check script |
| CSS/accessibility | design-tokens src/CSS | tests与产品视觉验证 |
| tgz问题 | `tools/sarmg_package_artifacts.py` | package tests/manifest |
| release-tree | `tools/sarmg_release/release.py` | release tests/asset builder |
| CI权限 | workflow policy脚本 | valid/invalid workflow fixture |
| 谁采用了什么 | `consumers/consumer-matrix.json` | 各产品manifest/lock/commit |

## 10.3 端到端练习一：管理员登录

从产品Web表单开始，逐步标记：

1. login候选何时由TS guard检查；
2. requestJson如何限制URL/credential/body；
3. Server adapter如何收集全部Origin/Host/authority/Sec-Fetch-Site；
4. Rust contract如何拒绝unknown field；
5. username/password 何时执行真正 policy；
6. Argon2 verifier和限流顺序；
7. token生成/摘要/Session表；
8. Cookie与Session JSON；
9. Web guard、private transport Session和UI Session；
10. 后续mutation的CSRF路径。

每一步写出Foundation保证、产品保证和一个负例。

## 10.4 端到端练习二：SQLite current identity

创建临时SQLite产品Schema，写canonical metadata，读取`sqlite_schema`行并计算fingerprint。然后分别改变：

- index SQL空白；
- metadata声明hash；
- application version；
- metadata列default；
- schema row顺序。

预测每项在哪个validator失败。不要通过自动覆盖metadata让测试通过。

## 10.5 端到端练习三：Package到消费者

1. clean/build contracts；
2. 查看dist公开文件；
3. pack真实tgz；
4. 审查tar member；
5. 创建不在monorepo内的空临时consumer；
6. offline install所有peer；
7. import根与Schema/fixture export；
8. 删除sibling仓库后再次运行；
9. 比较本地file lock与最终release URL lock。

目的是理解“source正确”“dist正确”“tgz正确”“consumer依赖正确”是四件事。

## 10.6 端到端练习四：认证竞态

构造可控fetch：restore阻塞时触发login；旧业务请求阻塞时完成logout/login；两次login按相反网络延迟返回。
写出每一步generation、public session、transport session、mutation tail和期望Promise结果。若无法预测，回读
admin-web源码和tests。

## 10.7 端到端练习五：Release攻击输入

在临时目录构造额外文件、错误mode、symlink、hardlink、path traversal manifest、hash时替换文件和超限树，
观察verifier应在哪一步fail fast。只操作临时目录，不在仓库或系统根创建危险链接。

## 10.8 术语表

| 术语 | 本项目中的精确含义 |
|---|---|
| build-time dependency | 编译/打包时取得并进入产品制品，生产不在线调用Foundation |
| consumer | 直接采用至少一个Foundation组件的真实产品仓库 |
| current-only | 一个发布只接受一个当前合同，不含历史fallback |
| wire contract | 跨进程/语言传输的字段、类型、边界和语义 |
| candidate | 通过基本结构但尚未被权威认证/业务校验的不可信输入 |
| runtime guard | JavaScript运行时把unknown验证并narrow为当前type的函数 |
| exact keys | required/allowed字段集合精确，不接受unknown |
| canonical | 同一语义只允许一个字节/文本表示 |
| primitive | 小而可组合、不拥有完整产品生命周期的共享能力 |
| fail closed | 信息缺失、歧义或无法验证时拒绝，而非猜测/降级 |
| typed error | 调用方可按variant/code处理，而非解析展示字符串 |
| same-origin | scheme、host、effective port均一致；本项目还要求完整header合同 |
| CSRF | Cross-Site Request Forgery；unsafe管理mutation需同源与当前token |
| PHC string | 密码散列的标准文本编码，包含algorithm/version/params/salt/hash |
| Argon2id | 当前管理员密码散列algorithm，参数仍必须精确匹配 |
| Session token | 32-byte随机值的43字符URL-safe Base64表示 |
| token digest | 产品持久化用于比较的SHA-256，而非raw token |
| Schema identity | application/version/revision/fingerprint四分量 |
| fingerprint | 对canonical排序的SQLite Schema原始字段做byte-exact SHA-256 |
| WAL | SQLite Write-Ahead Log；与checkpoint和备份一致性相关 |
| safe integer | JavaScript可精确表示的整数范围，不超过2^53-1 |
| peer dependency | 由消费者显式提供的精确package依赖 |
| package export | `package.json#exports`允许消费者使用的正式入口 |
| immutable source | 完整Git commit或不可覆盖release asset，而非branch/path |
| release identity | product/version/source/target/state contract hash五字段 |
| release tree | 路径/mode/size/hash精确且无额外文件的发布目录合同 |
| TOCTOU | 检查与使用之间对象被替换的竞态 |
| data plane | 设备、Agent、媒体流等业务通路，不等于管理员RBAC |
| control plane | 浏览器管理员配置/操作通路，当前只有admin角色 |
| compatibility alias | 为另一代名称/字段保留的额外入口，本仓明确不提供 |

## 10.9 学成后的评审能力

你应该能拒绝以下提案并给出具体理由：“让Foundation保存所有产品Session”“为了方便接受缺Origin”“泛型T
已经验证JSON”“Server也顺便支持ARM best effort”“把Dufs重写React才算统一”“把csrf放sessionStorage”
“发现missing DB就自动create”“release只要SHA256SUMS不用文件集合”“暂时保留旧字段以后再删”。

同时也应知道何时应共享：两个产品确实拥有同一 canonical username、token shape、Error Envelope、fingerprint
或React工具链，并且最强边界能被保留时，集中primitive和负例能显著提升长期可维护性。

## 10.10 后续入口

设计或修改共享能力时读[工作流程与流程树](../project-workflow.md)；逐项评估删除影响时查
[完整功能与取舍清单](../feature-inventory-and-tradeoffs.md)；准备tag、排查package或处理事件时以
[运维文档](../operations.md)为准。
