# Sarmg Foundation 完整功能与取舍清单

## 0. 读法、分类和验收规则

本清单是开发人员判断“Foundation 当前究竟实现了什么、删除后会发生什么、什么明确不属于它”的权威
边界台账，不是产品宣传页。每项都有唯一 ID、实现锚点、分类、复杂度、删除后果和最低验证/边界。修改
公开行为时，源码、测试、Schema/fixture、package/release、消费者和本表必须在同一大问题中同步。

分类只有五种：

- **核心**：定义 Foundation 身份、公开合同或不可替代的跨项目语义；删除会改变项目性质或直接破坏消费者。
- **保障**：控制认证、安全、完整性、并发、资源或供应链风险；通常不增加业务页面，但不可轻率删除。
- **可选**：消费者按需采用，不应成为隐式依赖；删除仍需先确认实际消费者。
- **建议保留**：产品理论上可自行实现，但共享能显著降低漂移和重复事故。
- **开发运维**：构建、测试、发布、审计、文档、追踪与事故响应能力。

复杂度按“重新设计 + 实现 + 正反测试 + 所有消费者迁移与发布验证”的总成本评为低/中/高，而不是代码
行数。认证、跨语言、持久状态、竞态、release-tree 和多仓库变更通常为高。

## 1. 项目身份、架构和 current-only 边界

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-001 | 纯 build-time 共享层，不运行中央 Foundation service | 根 README、Cargo/npm workspace、state contract 无资源 | 核心 | 高 | 引入生产网络依赖、中央故障域和所有产品锁步发布 | 消费者制品断网运行；无监听/daemon/config |
| FND-002 | 22 个 Rust crate 与 8 个 npm package 可独立选择 | `Cargo.toml` members、`pnpm-workspace.yaml`、各 manifest | 核心 | 中 | 小产品被迫引入 SQLx、React 或无关依赖，编译/审计面扩大 | manifest 只声明真实直接依赖；依赖树抽查 |
| FND-003 | 全组件统一版本 `0.5.0` | workspace version、package version、`foundation_policy.py` | 保障 | 中 | 同一 release 内无法确定可组合组件，lock 和支持矩阵失真 | repository policy 精确一致性检查 |
| FND-004 | 只提供唯一当前 API/合同/算法 | crate/package public API、strict guard、文档 | 核心 | 高 | 兼容分支和测试矩阵持续膨胀，产品边界不可证明 | 不存在 alias、dual reader/write、deprecated export |
| FND-005 | 历史迁移/备份/恢复归 `sarmg-upgrade` 与产品 adapter | README、合同边界、空 Foundation state | 核心 | 高 | 在线 runtime 被非当前解析器和高权限修改逻辑污染 | Foundation 不含 migration edge 或产品 DDL |
| FND-006 | Foundation 不拥有用户、Session、业务 DB、文件树或进程 | 所有 crate/package API 范围 | 核心 | 高 | 共享库变成特权平台，产品无法独立运行和发布 | API/依赖审计；state contract resources 为空 |
| FND-007 | 生产不依赖 GitHub/npm/Foundation 在线可用 | 消费者 pin、编译/打包模型 | 保障 | 高 | registry/GitHub 故障会让已部署产品不可用 | 构建后断网启动与核心功能验证 |
| FND-008 | 最强产品规则不得被通用 helper 削弱 | 接入流程、消费者集成测试 | 保障 | 高 | Sunshine TLS、产品路径/release verifier 等边界被最低共同实现替换 | 产品先/后置加强验证保留，攻击负例不减少 |
| FND-009 | Rust 全 workspace `unsafe_code=forbid` | `[workspace.lints.rust]` | 保障 | 中 | 新 unsafe 可绕过内存安全假设并扩大全仓审计 | all-target/all-feature check 与 lint |
| FND-010 | Clippy 禁止 `dbg!` 与 `todo!` | `[workspace.lints.clippy]` | 开发运维 | 低 | 临时诊断或未实现路径进入发布 crate | clippy `-D warnings` |
| FND-011 | Rust edition 2024、MSRV/toolchain 1.98 | workspace、`rust-toolchain.toml` | 开发运维 | 中 | 各 crate 编译语义和依赖解析漂移 | fixed toolchain check/test/doc |
| FND-012 | Web workspace Node 26.7.0、pnpm 10.12.1、TS 5.8.3 | `.node-version`、root/package manifest、lock | 开发运维 | 中 | 本地/CI/package 构建结果不一致 | policy + frozen install + package smoke |
| FND-013 | Apache-2.0 SPDX 元数据与审核文本一致；根 LICENSE 的 SHA-256 固定，22 个 crate 各携带普通单链接 byte-exact LICENSE，Cargo package 清单必须实际分发它，npm tgz也携带许可证 | workspace/package manifests、`LICENSE`、各 crate `LICENSE`、`foundation_policy.py`、`check-rust-package-licenses.py` | 保障 | 中 | `cargo vendor` 展平 Git dependency 后丢失许可证，第三方 notices 只能失败或错误借用消费者通用文本，发布来源和使用权不可审计 | 根摘要、链接/字节负例、22/22 `cargo package --list`、npm tar inventory、Dufs notices E2E |
| FND-014 | Foundation 无 `config/`、`deploy/`、`clients/` | 仓库布局 | 核心 | 低 | 容易误认为存在在线服务或产品 UI | 目录与 state contract 审查 |
| FND-015 | 发布 package 放 `packages/` 而非产品 `clients/web` | monorepo 布局 | 核心 | 低 | 发布依赖与可运行客户端职责混淆 | package exports 与消费者 import 验证 |

## 2. 管理员身份与密码：`sarmg-admin-auth`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-020 | 管理员身份只允许一个 canonical ASCII username 合同 | `require_canonical_administrator_username` | 核心 | 高 | 各产品对同一管理身份产生不同 key、比较、Session 与审计语义 | Rust/TS/Schema 共用正反 fixture |
| FND-021 | 登录候选只执行 `trim_ascii` + ASCII lowercase | `normalize_administrator_username` | 核心 | 中 | 产品可能执行 Unicode case folding 或其他改写，形成别名与限流绕过 | `" Admin.Ops " -> "admin.ops"`；Unicode/control 拒绝 |
| FND-022 | 登录候选必须是 1..64 printable ASCII bytes | normalize 入口、contracts Rust/TS/Schema | 保障 | 中 | 无界或控制字符身份会在 Argon2/账户查询前进入日志、内存与限流表 | empty/1/64/65/control/DEL/Unicode 边界 |
| FND-023 | canonical username 长度固定 3..64 bytes | `ADMINISTRATOR_USERNAME_MIN_BYTES/MAX_BYTES` | 保障 | 低 | 过短身份易混淆；无上限会放大 DB、日志、UI 和索引成本 | 2/3/64/65 bytes |
| FND-024 | canonical 字符只允许小写字母、数字、`.`、`_`、`-` | username validator、TS regex、JSON Schema | 保障 | 中 | 空格、引号、路径/邮箱符号会在 SQL、URL、日志或跨语言中产生歧义 | uppercase、`+`、`@`、空格、Unicode 负例 |
| FND-025 | canonical 首尾必须为 ASCII 字母或数字 | username validator、Schema pattern | 保障 | 低 | 首尾分隔符在复制、展示和配置中不易辨认且易被裁剪 | `.admin`、`admin_`、`-admin` 负例 |
| FND-026 | 管理身份明确不具有邮箱或 DNS domain 语义 | allowed set 不含 `@`，无 domain parser | 核心 | 中 | 重新加入邮箱会引入 local/domain、大小写和地址生命周期规则，并使当前各产品字段再次分叉 | `admin@example.test` 拒绝；代码/Schema 无 email 字段 |
| FND-027 | canonical 允许相邻中间分隔符且不折叠点/横线/下划线 | username validator 的字节精确比较 | 可选 | 低 | 若删除会缩小当前可用名称；若擅自折叠会把不同 username 合并 | `admin..ops`、`admin__ops` 正例；保持 byte-exact |
| FND-028 | 持久 username 与 Session 必须已 canonical，Foundation 不修复存量值 | `require_canonical_*` 与产品启动验证约定 | 保障 | 高 | 启动时静默改写身份会破坏 Session、审计和外键关联 | persisted uppercase/space 值启动失败且零写入 |
| FND-029 | 密码长度 12～1024 bytes | `validate_password`、常量 | 核心 | 中 | 过短降低安全；无上限可放大 Argon2 资源消耗 | 11/12/1024/1025 bytes |
| FND-030 | 密码禁止 ASCII control character | `validate_password` | 保障 | 中 | 换行/NUL 等造成配置、日志、wire 边界混淆 | 每类 control 与普通 Unicode 测试 |
| FND-031 | 密码长度按 UTF-8 bytes 而非字符数 | `password.len()` | 核心 | 中 | Rust/其他语言对多字节密码预算分叉 | 多字节边界 fixture |
| FND-032 | 新散列只使用 Argon2id | `current_argon2` | 核心 | 高 | 产品散列算法不同，安全与运维合同碎片化 | PHC algorithm 精确检查 |
| FND-033 | Argon2 PHC version 只接受 v19 | `Version::V0x13`、policy verifier | 核心 | 高 | verifier 接受其他语义版本，当前合同不唯一 | version 负例 |
| FND-034 | Argon2 memory 固定 19456 KiB | `ARGON2_MEMORY_KIB` | 核心 | 高 | 登录成本与容量规划跨产品漂移 | 参数减少/增加均拒绝 |
| FND-035 | Argon2 iterations 固定 2 | `ARGON2_ITERATIONS` | 核心 | 高 | 成本预算和 current hash 身份不一致 | `t` 参数负例 |
| FND-036 | Argon2 parallelism 固定 1 | `ARGON2_PARALLELISM` | 核心 | 高 | CPU 并行成本随实现变化 | `p` 参数负例 |
| FND-037 | Argon2 salt 固定 16-byte fresh random | `SaltString::generate`、policy verifier | 保障 | 高 | salt 重用/长度漂移降低或模糊安全合同 | 两次 hash 不同；非 16-byte拒绝 |
| FND-038 | Argon2 output 固定 32 bytes | `ARGON2_OUTPUT_BYTES` | 核心 | 高 | digest 长度不同却被误认为当前 hash | output length负例 |
| FND-039 | `hash_password` 先验证 plaintext policy | `hash_password` | 保障 | 中 | 非法密码仍被持久化为“合法 PHC” | 短/控制字符不生成 hash |
| FND-040 | `verify_password` 对非法 plaintext直接 false | `verify_password` | 保障 | 中 | 登录路径对无界/非法输入仍执行昂贵 verifier | 输入边界和 false 语义测试 |
| FND-041 | PHC 必须能 canonical round-trip | `hash.to_string() == encoded` | 保障 | 高 | 多种等价编码扩大持久状态和绕过检测面 | 非 canonical 编码拒绝 |
| FND-042 | 参数不同的有效 Argon2id hash也拒绝 | `password_hash_uses_current_policy` | 核心 | 高 | 在线 runtime 形成多 policy fallback | weak/alternate policy负例 |
| FND-043 | 可在无 plaintext 时检查持久 hash 当前性 | `require_current_password_hash` | 保障 | 高 | 服务只能在某管理员下次登录时才发现状态不合规 | 产品启动遍历所有管理员并 fail fast |
| FND-044 | Foundation 不实现“成功登录后顺便升级 hash” | API 明确无此入口 | 核心 | 高 | 在线服务携带历史转换和双 verifier | API 面/依赖扫描；转换归独立流程 |

## 3. Token、Cookie、same-origin 与 CSRF：`sarmg-admin-auth`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-050 | Session/CSRF token 使用 32-byte OS 随机源 | `random_token`、`getrandom::fill` | 保障 | 高 | token 可预测、跨产品熵不同 | 长度/随机源失败路径；无 fallback |
| FND-051 | token 使用 URL-safe Base64无 padding | `URL_SAFE_NO_PAD` | 核心 | 中 | Cookie/header/JSON 表示不一致 | 编码字符集与无 `=` 测试 |
| FND-052 | 当前 token 字符串恰好 43 bytes | `SESSION_TOKEN_ENCODED_BYTES`、`is_token_shape` | 核心 | 中 | 任意 raw token 或另一编码代进入 Session | 42/43/44 边界 |
| FND-053 | token 必须 decode 后再 canonical re-encode相同 | `is_token_shape` | 保障 | 高 | 非 canonical末尾 bits 或等价表示被接受 | 非规范最后字符负例 |
| FND-054 | token 摘要固定 SHA-256 32 bytes | `token_hash`、`TOKEN_HASH_BYTES` | 核心 | 高 | 产品 Session 表摘要算法/长度分叉 | golden digest 与长度测试 |
| FND-055 | 提供 lowercase hex token hash | `token_hash_hex` | 建议保留 | 低 | 配置型产品重复 hex 实现易漂移 | 64 位 lowercase hex |
| FND-056 | token 比较先验证 shape 与 expected hash长度 | `token_matches_hash` | 保障 | 高 | 任意字符串摘要可能被当当前 Session | noncanonical token/31/33-byte digest负例 |
| FND-057 | token digest 比较使用 constant time | `subtle::ConstantTimeEq` | 保障 | 高 | 普通比较暴露时序侧信道 | code review + match正反测试 |
| FND-058 | Cookie 名称仅允许字母数字/下划线/连字符 | `parse_cookie_value` | 保障 | 中 | 调用方用歧义名称解析 raw Cookie | 非法 name负例 |
| FND-059 | Cookie 只返回唯一非空同名值 | `parse_cookie_value` | 保障 | 高 | duplicate cookie fixation/选择顺序歧义 | 空值/重复值/空格分隔负例 |
| FND-060 | 框架必须自行拒绝重复 Cookie field line | `parse_cookie_value` doc contract | 保障 | 高 | helper只见被框架合并的一行而漏掉歧义 | 产品集成重复 header测试 |
| FND-061 | 安全 header helper 要求恰好一个 field-line value | `require_single_security_header_value` | 保障 | 高 | 框架挑第一/最后值造成 request smuggling语义 | missing/2行/3行 typed error |
| FND-062 | 安全 header 只接受 nonempty visible ASCII | 同上 | 保障 | 中 | 控制字符/空值进入 URL、日志或比较 | 空、space、control、non-UTF8负例 |
| FND-063 | 安全 header 拒绝逗号连接值 | 同上 | 保障 | 高 | 多值被代理合并后伪装成单值 | comma负例 |
| FND-064 | 同源校验显式区分 Production HTTPS 与 loopback HTTP dev | `AdministratorOriginMode` | 核心 | 高 | 私网明文或环境猜测成为生产 fallback | 两模式 scheme/host矩阵 |
| FND-065 | Production Origin scheme必须精确 `https` | `mode.scheme()` | 保障 | 高 | 管理凭据可经明文 HTTP | http production负例 |
| FND-066 | Development Origin scheme必须精确 `http` | `LoopbackDevelopmentHttp` | 核心 | 中 | 模式语义不唯一并掩盖代理错误 | https dev负例 |
| FND-067 | HTTP dev双方 host 必须是 localhost/127.0.0.0/8/::1 | `NormalizedHost::is_loopback` | 保障 | 高 | 私网/公网 HTTP 被误当开发例外 | 非loopback DNS/IP负例 |
| FND-068 | Origin、Host、Sec-Fetch-Site 都必须出现一次 | `require_administrator_same_origin` | 保障 | 高 | 缺失 header被静默接受，浏览器 CSRF 边界消失 | 三类 missing/duplicate负例 |
| FND-069 | `Sec-Fetch-Site` 必须精确 `same-origin` | 同上 | 保障 | 高 | cross-site/none/same-site 请求绕过 | 值枚举负例 |
| FND-070 | Origin 必须有且只有 `scheme://authority` | `split_once` + `parse_authority` | 保障 | 高 | path/query/fragment/userinfo等被 URL parser宽松接受 | 各 URL 部件攻击负例 |
| FND-071 | Host 支持 DNS、canonical IPv4、bracketed IPv6 | authority parser | 核心 | 高 | 各产品/IP版本对同源理解不同 | DNS/v4/v6正例 |
| FND-072 | DNS host比较时 lowercase，拒绝首尾点/非法 label | `parse_unbracketed_host` | 保障 | 高 | 等价或无效 host 产生不一致 authority | case正例、尾点/非法字符负例 |
| FND-073 | IPv4/IPv6 必须采用 canonical文本 | authority parser、`to_string` | 保障 | 高 | 多文本表示绕过 equality/log分析 | 非canonical IP负例 |
| FND-074 | 端口 1..65535、无前导零，缺失时用 scheme默认 | `parse_port`、`default_port` | 保障 | 高 | `:0443`、0、溢出或默认端口理解不一 | 端口边界矩阵 |
| FND-075 | Origin authority 与有效 Host authority精确相等 | `OriginHostMismatch` | 保障 | 高 | cookie/CSRF 发往被混淆 host/port | scheme默认端口与显式端口测试 |
| FND-076 | 调用方必须把 HTTP/2 URI authority加入有效 Host值集合 | API doc、消费者 adapter | 保障 | 高 | `Host` 与 `:authority` 冲突时框架静默选择 | authority-only正例、二者冲突负例 |
| FND-077 | 不解析 `X-Forwarded-*` 或信任代理 fallback | API 明确排除 | 核心 | 高 | 未经产品 trust boundary 的客户端可伪造外部 origin | API 面扫描；产品先形成权威 Host |
| FND-078 | CSRF header必须唯一、visible且为 canonical token | `require_single_csrf_token` | 保障 | 高 | 多值/任意 token进入比较 | missing/duplicate/comma/shape负例 |
| FND-079 | CSRF 与 Session digest常量时间比较 | `require_csrf_token_matches_hash` | 保障 | 高 | mutation 防护变成普通字符串比较 | match/mismatch/错误长度测试 |
| FND-080 | crate不决定 Cookie 名/flags/TTL/Session table | public API边界 | 核心 | 高 | 不同部署和威胁模型被错误锁死或中央化 | 产品拥有 Secure/HttpOnly/SameSite/撤销测试 |

## 4. 管理员与错误 wire contract：`sarmg-contracts` / `@sarmg/contracts` / `sarmg-error`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-090 | 三个管理员 auth path 是跨 Rust/TS常量 | `ADMIN_*_PATH`、`ADMIN_AUTH_PATHS` | 核心 | 高 | 产品 route/client产生双事实源 | 恰好 `/api/v2/auth/login|session|logout` |
| FND-091 | `AdministratorLoginRequest` exact 两字段，Rust 读写均先验证 | Rust custom serialize/deserialize、TS guard、Schema | 核心 | 高 | 隐藏字段/多角色/设备字段进入管理登录，或手工 struct 输出非法 wire | missing/unknown/type fixture；非法 public value 序列化失败 |
| FND-092 | login username 候选 1..64 printable ASCII bytes | Rust custom deserialize、TS guard、Schema | 保障 | 中 | 无界/Unicode/control body 在 Argon2/admission 前进入服务 | empty/64/65/control/DEL/Unicode 负例 |
| FND-093 | login password候选 1..1024 code points且无 control | 同上 | 保障 | 中 | wire先无界；与Server byte policy角色混淆 | empty/1025/control负例；Server再按bytes校验 |
| FND-094 | 管理角色枚举只有 `admin` | `AdministratorRole::Admin`、常量/Schema const | 核心 | 高 | 多角色扩展Schema、API、UI和授权状态空间 | viewer/operator/其他值拒绝 |
| FND-095 | Session恰好五字段 | `AdministratorSession`、guard、Schema | 核心 | 高 | 产品返回不同Session形状，Web auth分叉 | exact keys fixture |
| FND-096 | `authenticated` 必须为 literal true | custom deserialize/serialize、TS guard | 核心 | 中 | “未认证Session对象”进入已认证流程 | false/null/missing负例 |
| FND-097 | `user_id` 是有界ASCII identifier | contracts validator | 保障 | 中 | 任意展示文本/控制字符进入路由和日志 | 1..128、字符集边界 |
| FND-098 | Session username 必须已是 canonical 管理员用户名 | admin-auth复用、TS regex、Schema | 核心 | 高 | Server 与 Web 展示/比较不同身份或把登录候选误当持久值 | Rust/TS/Schema 同一 fixture；uppercase/`@`拒绝 |
| FND-099 | Session `csrf_token` 必须当前43字符token | admin-auth复用、TS guard、Schema | 保障 | 高 | Web持有无法被Server当前CSRF验证的值 | canonical末尾bits负例 |
| FND-100 | Rust Session constructor自动固定 `role=admin` | `AdministratorSession::new` | 保障 | 中 | 产品手写role或false authenticated | serialize验证和constructor测试 |
| FND-101 | `ErrorCode` 1..128 bytes、小写字母开头 | `sarmg-error::ErrorCode`、TS guard | 核心 | 中 | UI按自由文本分支，日志/指标字段无界 | 首字符/长度/Unicode负例 |
| FND-102 | ErrorCode只含小写字母、数字、`.`、`_`、`-` | ErrorCode validator | 保障 | 低 | 控制字符/空格进入日志、指标和client dispatch | 字符集 fixture |
| FND-103 | `RequestId` 1..128 bytes有界ASCII identifier | `RequestId`、TS `isRequestId` | 保障 | 中 | header/body/log correlation可注入或无界 | 空/129/control/Unicode负例 |
| FND-104 | Error Envelope拒绝unknown fields | Rust `deny_unknown_fields`、TS allowed keys、Schema | 核心 | 高 | 服务和client对错误含义静默分叉 | unknown/missing fixture |
| FND-105 | Error Envelope固定 code/message/retryable | `ErrorEnvelope` | 核心 | 中 | client退回解析展示文案或猜可重试性 | required字段正反测试 |
| FND-106 | `request_id` 缺失与存在有效值区分，显式null拒绝 | custom deserialize、guard/Schema | 保障 | 中 | null/非法ID被当缺失，关联证据失真 | missing/null/invalid fixture |
| FND-107 | `details` 只能是对象，空对象序列化省略 | `Map<String,Value>`、guard/Schema | 保障 | 中 | raw任意JSON/诊断泄露，shape不稳定 | array/scalar负例；empty serialization |
| FND-108 | message只用于展示，machine分支使用code/status | 类型doc与http-client | 核心 | 中 | 文案调整破坏业务逻辑和国际化 | 消费者不按message分支审查 |
| FND-109 | 9个常用HTTP status与code映射 | `HttpStatus` | 建议保留 | 低 | 各产品重复映射并可能把所有失败变500 | 400/401/403/404/409/422/429/500/503测试 |
| FND-110 | 默认仅429/503可重试 | `default_retryable` | 保障 | 中 | client可能重放有副作用请求或永不重试临时失败 | status映射测试；产品可显式覆盖 |
| FND-111 | Rust error类型可构造、parse、display与serde | impl集合 | 建议保留 | 中 | 产品复制有界identifier实现 | round-trip与invalid serde测试 |
| FND-112 | Rust/TS/Error Schema共用相同fixture | contracts fixtures + Rust include | 保障 | 高 | 错误路径跨语言漂移只在生产出现 | valid/invalid在两端全部执行 |

## 5. State、Release 与 Backup 合同

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-120 | State Contract wire version固定1 | `STATE_CONTRACT_VERSION`、Schema/guard | 核心 | 高 | 工具无法判断状态描述语义 | 其他version拒绝 |
| FND-121 | State exact fields且unknown拒绝 | Rust serde、TS guard、Schema | 核心 | 高 | 产品遗漏/扩展被另一端静默忽略 | missing/unknown fixture |
| FND-122 | application/version为有界identifier | contract validators | 核心 | 中 | 产品身份不能稳定进入release/backup | 字符与长度边界 |
| FND-123 | source_revision恰好40位小写hex | custom deserialize、TS guard、Schema | 保障 | 中 | release无法绑定一个精确Git对象 | 39/41/uppercase/nonhex负例 |
| FND-124 | State schema字段必须出现，可为null | custom required option、guard exact keys | 核心 | 高 | “无DB”与“忘记声明”混淆 | missing/null/object三态fixture |
| FND-125 | State schema revision为非负safe integer | custom safe integer、guard | 保障 | 高 | JS静默失真或负版本进入工具 | MAX_SAFE/+1/fraction/negative负例 |
| FND-126 | State schema SHA为64位小写hex | validator | 核心 | 中 | Schema身份不可复核 | hash边界fixture |
| FND-127 | maintenance lock为唯一identifier数组 | unique validator | 保障 | 高 | 重复/歧义lock导致工具顺序错误 | duplicate/invalid lock负例 |
| FND-128 | State resource kind只有5种当前值 | `StateResourceKind`、TS union、Schema | 核心 | 高 | 工具对未知持久资源做错误处理 | unknown kind拒绝 |
| FND-129 | State resource恰好name/kind/required | struct/guard/Schema | 核心 | 中 | 必需资源语义被遗漏或附带未知字段 | exact-field fixture |
| FND-130 | External requirement描述kind/kid/algorithm/envelope_version | contract struct/guard | 核心 | 高 | backup/restore不知道缺哪个外部Secret合同 | identifier与positive safe integer测试 |
| FND-131 | Companion contract绑定name/version/platform/SHA | contract struct/guard | 核心 | 高 | native companion可与Server状态不匹配 | exact fields/hash fixture |
| FND-132 | Release Identity恰好五字段 | `ReleaseIdentity`、Schema/guard | 核心 | 高 | 制品身份出现多套格式或升级字段泄入 | exact key fixture |
| FND-133 | Release product/version/target为identifier | validators | 核心 | 中 | 发布工具对路径/平台身份解释不同 | invalid identifier负例 |
| FND-134 | Release绑定完整source revision | `source_revision` | 保障 | 高 | 同版本制品不能追溯源码 | 40位SHA与tag/HEAD复核 |
| FND-135 | Release用state_contract_sha256绑定状态合同 | identity字段、asset builder | 保障 | 高 | 制品与状态/恢复合同可被独立替换 | build顺序和hash drift负例 |
| FND-136 | Release不含升级工具版本/edge | exact contract | 核心 | 中 | Foundation release被历史迁移矩阵耦合 | unknown字段拒绝 |
| FND-137 | Backup Manifest wire version固定2 | `BACKUP_MANIFEST_VERSION` | 核心 | 高 | 离线工具无法确定备份语义 | 其他version拒绝 |
| FND-138 | Backup exact fields且unknown拒绝 | Rust/TS/Schema | 核心 | 高 | 恢复端忽略重要字段或接受模糊数据 | shared fixture |
| FND-139 | Backup schema identity复用完整四字段SchemaIdentity | type alias与Schema | 核心 | 高 | 备份身份和DB身份算法复制漂移 | null/object及exact-current由产品加强 |
| FND-140 | Backup created_at为非负safe epoch seconds | safe integer validator | 保障 | 中 | JS失真或负时间破坏审计 | max/+1/negative/fraction负例 |
| FND-141 | Backup resources至少一项 | custom deserialize/validate、guard | 保障 | 高 | 空归档被标记成功备份 | empty array负例 |
| FND-142 | Backup resource path非空但不声称canonical | `validate_non_empty_path` | 核心 | 中 | 通用层若猜路径规则会弱化产品no-follow/布局 | empty拒绝；产品继续验证canonical/path root |
| FND-143 | Backup bytes非负safe、files正safe integer | validators | 保障 | 高 | 资源计数溢出/空文件集合被当有效 | 0 files、unsafe integer负例 |
| FND-144 | Backup每个资源有SHA-256 | `BackupResource` | 保障 | 高 | manifest无法绑定实际资源内容 | hash格式 + 产品实际hash复核 |
| FND-145 | Backup external requirement额外绑定Secret材料SHA | `BackupExternalRequirement` | 保障 | 高 | key ID相同但材料不同仍被误用 | sha/algorithm/envelope exact测试 |
| FND-146 | Rust State/Release/Backup直接include TS package fixture | `include_str!`测试 | 保障 | 高 | 两语言各自测试但语义仍漂移 | 同一valid/invalid集合 |
| FND-147 | JSON Schema与fixture作为公开package export | contracts manifest/copy script | 建议保留 | 中 | 非Rust/TS工具复制合同或深层导入 | tarball export解析 |

## 6. Schema 身份：`sarmg-schema-identity`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-150 | Schema算法与数据库driver解耦 | crate无SQLx/rusqlite dependency | 核心 | 高 | SQLx/rusqlite native link冲突或复制算法 | dependency树、双driver消费者 |
| FND-151 | canonical `product_metadata` DDL恰好五列 | `PRODUCT_METADATA_DDL` | 核心 | 高 | 产品metadata字段/约束分叉 | DDL fixture与产品migration对比 |
| FND-152 | singleton INTEGER PK NOT NULL CHECK=1 | DDL/column validator | 保障 | 高 | 多身份row或无唯一锚点 | DDL/PRAGMA/row负例 |
| FND-153 | application/application_version TEXT NOT NULL | DDL/column validator | 核心 | 中 | SQLite动态类型让身份变为null/其他storage class | column和runtime typeof检查 |
| FND-154 | schema_revision INTEGER NOT NULL | DDL/column validator | 核心 | 中 | revision形态漂移 | column/storage/negative测试 |
| FND-155 | schema_sha256 TEXT NOT NULL | DDL/column validator | 核心 | 中 | hash以blob/null等形式持久化 | column/storage/hash测试 |
| FND-156 | 五列顺序/cid/type/notnull/pk/default精确验证 | `validate_product_metadata_columns` | 保障 | 高 | 同名但不同shape的表被接受 | 每个列属性漂移负例 |
| FND-157 | DDL比较只忽略ASCII空白与字母case | `validate_product_metadata_ddl` | 核心 | 高 | 语义不同DDL被过度normalize为相同 | check/default/列变化负例 |
| FND-158 | metadata必须恰好一row | `schema_identity_from_metadata_rows` | 保障 | 高 | 任意选择0/多row身份 | 0/2 row typed error |
| FND-159 | singleton值必须精确1 | `ProductMetadataRow::to_schema_identity` | 保障 | 中 | 错误row被当产品身份 | 0/2负例 |
| FND-160 | schema revision不能为负 | i64→u64 checked conversion | 保障 | 中 | SQLite负数绕过wire非负规则 | -1负例 |
| FND-161 | identity四分量都参与exact current比较 | `SchemaIdentity::require_exact` | 核心 | 高 | 只比hash/revision会接纳其他产品/版本DB | 每字段mismatch typed error |
| FND-162 | identifier 1..128 ASCII安全字符 | identity validator | 保障 | 中 | 产品/version进入日志/manifest时无界或注入 | 字符/长度测试 |
| FND-163 | schema SHA必须64位lowercase hex | `validate_schema_sha256` | 核心 | 中 | 非canonical或错误hash进入身份 | upper/length/nonhex负例 |
| FND-164 | fingerprint算法版本常量为1 | `SCHEMA_FINGERPRINT_ALGORITHM_VERSION` | 核心 | 高 | 不同实现无法声明字节算法 | golden vector版本 |
| FND-165 | 查询排除 `sqlite_*` 内部对象 | `SQLITE_SCHEMA_ROWS_QUERY` +函数二次拒绝 | 保障 | 高 | SQLite版本/内部对象改变产品hash | excluded row负例 |
| FND-166 | 查询排除 `product_metadata` 自身 | query +函数二次拒绝 | 核心 | 高 | metadata声明hash与hash输入自引用 | included metadata负例 |
| FND-167 | 行按 type/name/table的BINARY顺序 | query ORDER BY +函数检查 | 核心 | 高 | adapter/locale/返回顺序改变hash | 乱序负例 |
| FND-168 | duplicate Schema object key拒绝 | `DuplicateSchemaObject` | 保障 | 高 | 重复输入被无声hash入且adapter差异 | duplicate负例 |
| FND-169 | 每row四字段分别加入u64 big-endian字节长度 | `schema_fingerprint` framing | 核心 | 高 | 简单拼接产生边界碰撞或跨语言分叉 | golden framing vectors |
| FND-170 | SQL按原始UTF-8 bytes进入hash | `digest.update(bytes)` | 核心 | 高 | formatter/语义归一化把真实DDL drift合并 | 空白/Unicode SQL差异vector |
| FND-171 | 声明fingerprint与实际fingerprint双重比较 | `verify_fingerprint`/`verify_current_schema` | 保障 | 高 | 只改metadata即可伪装当前DB | declared/actual mismatch负例 |
| FND-172 | 发布可复核golden vectors JSON | crate fixture常量 | 开发运维 | 中 | 新driver/语言无法独立证明同算法 | fixture parse与expected hash |
| FND-173 | typed error指出row/column/identity/order/hash字段 | `Error`/`IdentityField` | 建议保留 | 中 | doctor只能解析字符串，现场定位困难 | error variant单元测试 |

## 7. SQLx SQLite 基线：`sarmg-sqlite`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-180 | `open_existing` 明确拒绝缺失数据库 | path `try_exists` + create=false | 保障 | 高 | 拼错路径会被SQLite悄悄创建空库 | missing typed error；产品先做no-follow |
| FND-181 | `create_if_missing` 是单独显式入口 | create=true API | 核心 | 高 | 初始化意图与正常启动混淆 | missing创建、existing打开测试 |
| FND-182 | pool max connections必须大于0 | `PoolOptions::validate` | 保障 | 低 | 无效pool在运行时产生难懂错误 | 0负例 |
| FND-183 | min connections不能超过max | 同上 | 保障 | 低 | 配置自相矛盾 | 边界负例 |
| FND-184 | acquire timeout必须非零，默认10秒 | options/常量 | 保障 | 中 | 无限/立即等待语义跨产品漂移 | zero/default/custom测试 |
| FND-185 | 每连接强制foreign_keys=ON | `SqliteConnectOptions` | 保障 | 高 | 新连接可写悬空引用 | 多连接PRAGMA与FK violation |
| FND-186 | 每连接强制WAL | journal mode | 建议保留 | 高 | 读写并行、checkpoint和备份语义分叉 | PRAGMA实际值 |
| FND-187 | 每连接强制synchronous=FULL | synchronous mode | 保障 | 高 | 断电持久窗口扩大 | PRAGMA实际值 |
| FND-188 | busy timeout固定5秒 | `BUSY_TIMEOUT` | 保障 | 中 | 锁冲突随机立即失败或无限等待 | PRAGMA/锁竞争测试 |
| FND-189 | integrity check必须唯一返回`ok` | `integrity_check` | 保障 | 高 | 损坏DB继续运行或进入备份 | 多诊断/损坏fixture |
| FND-190 | FK check收集table/row/parent/index | `ForeignKeyViolation` | 保障 | 高 | 只知道失败无法定位或完全漏检 | 具体violation断言 |
| FND-191 | TRUNCATE checkpoint busy是失败 | `CheckpointBusy` | 保障 | 高 | 仍有reader时只复制main file造成不完整备份 | busy reader测试 |
| FND-192 | checkpoint frame不完整也是失败 | `CheckpointIncomplete` | 保障 | 高 | 部分WAL被误当安全checkpoint | tuple validator负例 |
| FND-193 | schema rows可从pool/connection/transaction executor读取 | generic `Executor<Sqlite>` | 建议保留 | 中 | 产品重复adapter并绕开事务一致视图 | 三类调用/编译测试 |
| FND-194 | SQLx adapter复用纯fingerprint算法 | `fingerprint_rows` | 核心 | 高 | SQLx与离线rusqlite工具hash分叉 | shared golden/current schema测试 |
| FND-195 | 读取metadata前同时校验DDL、列与storage class | `validate_metadata_table`/`typeof` query | 保障 | 高 | SQLite动态类型或近似表冒充当前合同 | DDL/PRAGMA/typeof负例 |
| FND-196 | `read_schema_identity` 先验证实际hash才返回身份 | function顺序 | 保障 | 高 | 调用方误信未绑定真实DDL的metadata | drifted DDL测试 |
| FND-197 | pool convenience仍执行相同current验证 | `read_pool_*`/`require_pool_*` | 建议保留 | 低 | 产品为便利绕过connection级验证 | wrapper集成测试 |
| FND-198 | 不含业务DDL、migration、路径权限、实例锁、backup | crate public API | 核心 | 高 | 通用层越权修改产品状态或给出错误安全保证 | API面审计；产品生命周期测试 |

## 8. Server 架构门禁：`sarmg-server-target`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-200 | Server唯一target为`x86_64-unknown-linux-gnu` | `SERVER_TARGET_TRIPLE` | 核心 | 高 | 各产品对“支持平台”有不同声明和制品 | 常量、release identity、文档一致 |
| FND-201 | target arch必须x86_64 | compile-time cfg | 保障 | 高 | ARM Server可能未经等价验证被发布 | aarch64 compile-fail |
| FND-202 | target OS必须Linux | compile-time cfg | 保障 | 高 | Windows/macOS Server绕过Linux文件/服务假设 | cross-OS compile-fail |
| FND-203 | target env必须GNU/glibc | compile-time cfg | 保障 | 高 | musl制品与动态加载/部署合同不符 | musl compile-fail |
| FND-204 | pointer width必须64 | compile-time cfg | 保障 | 中 | 32位资源/ABI边界未验证 | cfg gate |
| FND-205 | 非目标在编译期直接`compile_error!` | crate root | 保障 | 高 | 仅依赖CI约定，开发者可本地误构建 | negative target job/手动编译 |
| FND-206 | 提供human architecture常量`amd64` | `SERVER_ARCHITECTURE` | 建议保留 | 低 | release/install脚本各自命名架构 | 常量测试 |
| FND-207 | runtime/release metadata可精确检查target | `require_server_target` | 保障 | 中 | manifest宣称与编译target漂移 | accepted canonical、所有近似值拒绝 |
| FND-208 | 客户端/Agent不依赖该crate | crate文档与消费者Cargo边界 | 核心 | 高 | Host/移动/桌面客户端被误限制为AMD64 Linux | dependency tree + 各客户端平台构建 |
| FND-209 | Foundation自身不宣称是AMD64 Server | release target `source-any` | 核心 | 中 | source package和Server平台概念混淆 | release identity检查 |

## 9. JSON HTTP Client：`@sarmg/http-client`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-220 | `requestJson<T>` 是唯一请求入口 | package export | 核心 | 中 | 产品fork多个不同安全helper | export/tarball测试 |
| FND-221 | 无浏览器location时必须显式绝对baseUrl | `resolveSameOriginUrl` | 保障 | 中 | Node测试/SSR错误解析相对URL | no-location负例 |
| FND-222 | URL只允许HTTP/HTTPS | `parseHttpUrl` | 保障 | 高 | data/file/javascript等scheme进入fetch | scheme负例且fetch未调用 |
| FND-223 | URL拒绝控制字符和空值 | `parseHttpUrl` | 保障 | 中 | 日志/解析混淆 | control/empty负例 |
| FND-224 | URL拒绝userinfo | username/password检查 | 保障 | 高 | credential嵌入URL并可能泄漏 | userinfo负例 |
| FND-225 | target origin与base origin精确相等 | origin比较 | 保障 | 高 | cookie/CSRF请求发往第三方 | scheme/host/port变化负例 |
| FND-226 | redirect固定`error`且不允许caller放宽 | request validation/fetch init | 保障 | 高 | 30x绕过前置same-origin检查 | override负例、capture init |
| FND-227 | credential默认`same-origin` | fetch init | 核心 | 中 | Session cookie不发送或策略各产品漂移 | fetch capture |
| FND-228 | 默认Accept application/json，保留其他header | Headers logic | 建议保留 | 低 | 内容协商不稳定或caller header丢失 | header merge测试 |
| FND-229 | method必须string并统一uppercase判断安全性 | method validation | 保障 | 中 | 非法method或大小写绕过CSRF分类 | method type/case测试 |
| FND-230 | 只对unsafe method注入CSRF | `isUnsafeMethod` | 保障 | 高 | token泄入safe request或mutation漏token | GET/HEAD/OPTIONS/TRACE vs POST/PUT/PATCH/DELETE |
| FND-231 | CSRF必须非空且无CR/LF | token validation | 保障 | 高 | header injection或空防护 | empty/newline负例 |
| FND-232 | timeout默认10秒、范围1..120000ms safe integer | constants/`validateBudget` | 保障 | 中 | 请求无限挂起或配置溢出 | 上下界/fraction/NaN负例 |
| FND-233 | caller AbortSignal与timeout first-wins合并 | local AbortController/source | 保障 | 高 | 错误分类竞态随机，listener泄漏 | caller-first/timeout-first交错测试 |
| FND-234 | finally清timer和caller listener | `finally` | 保障 | 中 | 长期页面积累资源泄漏 | fake timer/listener测试 |
| FND-235 | network/timeout/caller abort有不同typed code | catch/localError | 核心 | 高 | UI误报或错误重试mutation | 三类失败测试 |
| FND-236 | 成功响应默认2MiB、最大64MiB | constants/budget validation | 保障 | 高 | `response.json()`无界占内存 | exact/+1和配置上限测试 |
| FND-237 | 错误正文独立固定64KiB上限 | `MAX_ERROR_RESPONSE_BYTES` | 保障 | 高 | 大型代理错误页耗尽内存 | oversize error测试 |
| FND-238 | 先拒绝过大声明Content-Length并取消body | `readBoundedText` | 保障 | 高 | 已知超限仍读取全部内容 | declared size/cancel测试 |
| FND-239 | 再流式累计实际Uint8Array bytes | reader/chunks | 保障 | 高 | chunked/错误Content-Length绕过预算 | chunked exact/+1测试 |
| FND-240 | 超限时reader cancel失败不覆盖权威size error | nested try/catch | 保障 | 中 | transport cleanup异常改变错误分类 | cancel throw测试 |
| FND-241 | UTF-8使用fatal decode | `TextDecoder(...,{fatal:true})` | 保障 | 中 | replacement char使JSON/文本静默变化 | malformed UTF-8测试 |
| FND-242 | success/error只接受JSON或`+json`Content-Type | `isJsonContentType` | 保障 | 中 | HTML/文本错误页进入业务状态或UI | MIME参数、html/plain负例 |
| FND-243 | HEAD/204/205/Content-Length 0返回undefined | empty response分支 | 核心 | 中 | logout/no-content被误解析为invalid JSON | 各空响应测试 |
| FND-244 | 非2xx严格解析共享Error Envelope | `responseError` + guard | 核心 | 高 | client按任意upstream body分支 | valid/invalid envelope测试 |
| FND-245 | 非法error body不回显raw HTML/Secret | safe generic error | 保障 | 高 | 内部诊断或Secret泄入UI/log | sentinel secret不可见断言 |
| FND-246 | body request_id优先、header fallback且均严格过滤 | safeRequestId/responseError | 保障 | 中 | correlation ID注入或错误关联 | body/header优先级与非法值测试 |
| FND-247 | Retry-After支持safe整数秒并封顶24h | parser | 建议保留 | 中 | 各UI等待提示溢出/理解不同 | 0/large/unsafe测试 |
| FND-248 | Retry-After日期只接受规范HTTP-date round-trip | regex/Date/UTCString | 保障 | 中 | 宽松日期解析跨runtime不一致 | canonical/noncanonical日期测试 |
| FND-249 | 401 callback被await | response path | 保障 | 高 | Session清理与路由竞态不可控 | async callback顺序测试 |
| FND-250 | 401 callback异常不覆盖权威API error | callback catch | 保障 | 高 | 本地cleanup错误吞掉Server 401证据 | throwing callback测试 |
| FND-251 | 从不自动重试 | API无retry loop | 核心 | 高 | 未知副作用mutation被重复执行 | fetch调用次数负例 |
| FND-252 | 泛型T不声称运行时验证 | JSON parse cast + docs | 核心 | 中 | 调用方把静态类型当网络信任边界 | 产品guard集成测试 |
| FND-253 | 不用于文件上传/下载或streaming媒体 | package边界 | 核心 | 高 | JSON预算/parse破坏大对象业务 | API面与产品专用transport |

## 10. 管理员 Web：`@sarmg/admin-web`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-260 | 统一精确Node/React/Vite/TS工具链常量 | `ADMIN_WEB_TOOLCHAIN` | 核心 | 高 | 四个React产品依赖版本和bundle行为漂移 | 每个值与manifest/toolchain测试 |
| FND-261 | manifest assertion检查Node engine精确字符串 | `assertAdministratorWebToolchain` | 保障 | 中 | 产品声明宽松engine掩盖本地/CI差异 | 任何范围变化拒绝 |
| FND-262 | `.node-version`只接受精确值可带单个换行 | 同上 | 保障 | 低 | 工作站工具链与package engine分叉 | exact/extra文本负例 |
| FND-263 | dependencies/dev/peer/optional中出现工具链包就必须精确值 | manifest遍历 | 保障 | 高 | 通过另一section藏caret/不同patch | 每section drift负例 |
| FND-264 | 不强制消费者使用pnpm | assertion只检查工具链，不查packageManager | 核心 | 中 | npm消费者被无意义迁移或双lock | Host/Sunshine/Media/Sentinel npm lock验证 |
| FND-265 | 浏览器默认baseUrl为runtime origin根 | `resolveAdministratorBaseUrl` | 核心 | 中 | 各Web对相对auth路径理解不同 | browser location测试 |
| FND-266 | 浏览器显式baseUrl仍必须同origin | base/runtime origin比较 | 保障 | 高 | 管理Cookie/CSRF被发送到第三方 | cross-origin负例 |
| FND-267 | baseUrl只允许HTTP(S)且无userinfo | URL validation | 保障 | 高 | 非网络scheme或嵌入凭据 | protocol/userinfo负例 |
| FND-268 | Node/nonbrowser必须显式baseUrl | runtime location guard | 保障 | 中 | 测试环境隐式依赖全局location | no-location负例 |
| FND-269 | 业务path必须以`/api/v2/`开头 | `resolveApiPath` | 保障 | 高 | shared client被用来访问任意页面/origin | prefix/control/hash负例 |
| FND-270 | path解析后仍检查origin与pathname | URL revalidation | 保障 | 高 | `//host`、编码或URL解析技巧越界 | origin/prefix攻击负例 |
| FND-271 | 调用方不能自设`X-CSRF-Token` | send header ownership检查 | 保障 | 高 | stale/伪造token绕过当前Session所有权 | existing header负例 |
| FND-272 | body存在且无Content-Type时自动JSON | send header逻辑 | 建议保留 | 低 | 各产品遗漏JSON content type | capture测试；保留caller明确值 |
| FND-273 | credential只能未设置或same-origin | send init检查 | 保障 | 高 | caller改成include/omit破坏auth模型 | override负例 |
| FND-274 | 成功响应必须通过调用方guard | `send<T>` | 保障 | 高 | 任意JSON进入React状态 | false guard产生`invalid_response_shape` |
| FND-275 | Session/CSRF仅保存在closure内存 | `session`/`transportSession`局部变量 | 保障 | 高 | XSS后长期token残留、跨tab/重启语义扩大 | 源码扫描无storage；重载需restore |
| FND-276 | publish对所有订阅者同步通知并可unsubscribe | listener Set | 建议保留 | 中 | 产品auth store实现分叉或泄漏listener | subscribe/unsubscribe测试 |
| FND-277 | login request先经过共享候选guard | `isAdministratorLoginRequest` | 保障 | 中 | 无界/控制字符先发到Server | client拒绝且fetch未调用 |
| FND-278 | login/logout认证mutation全局串行 | `authenticationMutationTail` | 保障 | 高 | Set-Cookie网络完成顺序与UI调用顺序冲突 | overlap顺序测试 |
| FND-279 | 每次login/logout/当前401推进generation | `authenticationGeneration` | 保障 | 高 | 较旧响应复活或清除较新Session | delayed response竞态测试 |
| FND-280 | login开始立即清旧UI Session | `publish(null)` | 保障 | 中 | 凭据切换时旧授权页面继续操作 | transition测试 |
| FND-281 | `transportSession`只服务排队logout的CSRF | private snapshot | 保障 | 高 | login后立刻logout可能无法撤销Server Cookie，或出现双授权状态 | overlapping login/logout测试 |
| FND-282 | superseded auth operation返回typed client error | `auth_operation_superseded` | 核心 | 中 | UI无法区分当前失败与过期结果 | generation测试 |
| FND-283 | 并发restore复用同一Promise | `restorePromise` | 保障 | 高 | 页面多组件启动造成重复Session轮换/请求 | fetch调用一次测试 |
| FND-284 | restore等待之前排队的auth mutation | `precedingMutations` | 保障 | 高 | restore与Set-Cookie login/logout乱序 | overlap测试 |
| FND-285 | restore仅在当前generation发布Session | `requireCurrentOperation` | 保障 | 高 | 延迟restore覆盖新login/logout | delayed restore测试 |
| FND-286 | invalid Session response清当前状态 | restore catch code集合 | 保障 | 高 | 服务漂移后UI继续信任本地Session | shape/content/json/size错误测试 |
| FND-287 | 401只在dispatch Session仍为current时invalidate | auth context比较 | 保障 | 高 | 旧请求延迟401登出新Session | stale 401竞态测试 |
| FND-288 | logout本地授权同步结束，Server请求仍排队完成 | logout流程 | 保障 | 高 | 用户点击退出后仍短暂可操作，或Cookie不撤销 | immediate state + network顺序测试 |
| FND-289 | logout finally清transport Session | finally | 保障 | 中 | 失败后私有CSRF快照继续存在 | network failure测试 |
| FND-290 | React hook提供四态discriminated union | `AdministratorSessionState` | 核心 | 中 | 页面以多个boolean组合出非法auth状态 | TS类型与render测试 |
| FND-291 | hook mount自动restore | `useEffect` | 建议保留 | 中 | 产品各自遗漏Session恢复或闪烁逻辑 | lifecycle测试 |
| FND-292 | hook用active client ref阻止换client后的旧更新 | `activeClient` | 保障 | 高 | tenant/base/client切换后旧响应污染新页面 | client swap测试 |
| FND-293 | hook用state generation阻止旧Promise覆盖 | `stateGeneration` | 保障 | 高 | React异步login/restore竞态 | interleaving测试 |
| FND-294 | hook把401/superseded归anonymous，其余restore失败归error | catch分类 | 核心 | 中 | 网络故障被误显示为“未登录”或反之 | error分类测试 |
| FND-295 | Vite helper固定React plugin、dist与emptyOutDir | `createSarmgReactViteConfig` | 开发运维 | 中 | 产品构建插件/输出和stale asset策略漂移 | config对象与产品build检查 |
| FND-296 | Vite helper只允许产品选择base | `SarmgReactViteOptions` | 核心 | 中 | 共享配置越权控制产品业务bundle | public API审查 |
| FND-297 | Dufs明确不采用React/Vite helper | 项目级例外与root差异文档 | 核心 | 高 | 为表面统一重写成熟嵌入前端，扩大风险 | Dufs保持ES modules但共享auth合同 |

## 11. 设计 primitive：`@sarmg/design-tokens`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-300 | TS导出小型颜色/间距/圆角primitive | `src/index.ts` | 可选 | 低 | 程序化图表/样式复制magic value | declaration/export/test |
| FND-301 | light语义token与primitive对齐 | `semanticTokens.light`、`tokens.css` | 建议保留 | 中 | 多Web同义背景/文字/action命名漂移 | source/CSS effective value测试 |
| FND-302 | dark只覆盖真正变化的语义值 | `semanticTokens.dark`、`tokens.dark.css` | 建议保留 | 中 | 暗色漏项或复制整套primitive | light+override effective测试 |
| FND-303 | CSS变量统一`--sarmg-*` namespace | token CSS | 保障 | 中 | 宿主页或产品变量冲突 | selector/property扫描 |
| FND-304 | reset只在`[data-sarmg-scope]`内生效 | `reset.css` | 保障 | 高 | 全局污染Dufs/嵌入页/第三方内容 | 禁止无scope全局selector测试 |
| FND-305 | box-sizing对scope及后代统一 | scoped reset | 建议保留 | 低 | 浏览器布局差异回到每个产品 | CSS source/dist测试 |
| FND-306 | 表单继承font且disabled cursor明确 | reset | 建议保留 | 低 | 控件视觉/交互基线漂移 | CSS规则测试 |
| FND-307 | heading/text wrap与hidden基线 | reset | 建议保留 | 低 | 长身份/错误溢出或hidden失效 | CSS规则测试 |
| FND-308 | `:focus-visible`提供清晰键盘焦点 | accessibility CSS | 保障 | 中 | 键盘用户无法定位焦点 | CSS + 产品键盘测试 |
| FND-309 | reduced-motion显著压缩动画/transition | media query | 保障 | 中 | 动态敏感用户无法使用 | media query/source-dist测试 |
| FND-310 | forced-colors使用系统Highlight | media query | 保障 | 中 | 高对比度模式焦点不可见 | CSS + 人工高对比度验证 |
| FND-311 | visually-hidden保留辅助技术文本 | utility class | 建议保留 | 低 | 产品重复易错的屏幕阅读器隐藏模式 | exact CSS测试 |
| FND-312 | clean build删除陈旧CSS/JS声明 | clean/copy scripts | 开发运维 | 中 | 已删token仍在tarball可被深层使用 | stale artifact负例 |
| FND-313 | 不包含组件、品牌、字体、主题状态或CDN | package边界 | 核心 | 高 | 所有Web被单一视觉release cadence耦合 | API/tar inventory/network扫描 |

## 12. Package、发布树与供应链

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-320 | 8个npm package统一metadata/license/engine/repository | manifests + policy | 开发运维 | 中 | 来源、许可、工具链不可审计 | manifest check |
| FND-321 | package只发布`dist` | `files:["dist"]` | 保障 | 中 | src/test/cache/Secret意外进入tar | tar inventory |
| FND-322 | 每个公开入口显式写入exports | package manifests | 核心 | 中 | 消费者深层导入私有布局或入口漏包 | import.meta.resolve全部export |
| FND-323 | contracts复制5份Schema与5份fixture | copy-contract-data script | 核心 | 中 | 机器合同或跨语言测试数据缺失 | dist/tar export检查 |
| FND-324 | admin-web复制共享tsconfig公开入口 | copy-config script | 建议保留 | 中 | 产品strict编译基线复制漂移 | export解析与内容测试 |
| FND-325 | design-token复制4份CSS入口 | copy-css script | 核心 | 中 | package声明与实际CSS缺失 | source=dist/tar测试 |
| FND-326 | build前clean且拒绝linked dist | package artifact policy | 保障 | 高 | 陈旧输出或symlink逃逸进入包 | linked/stale dist负例 |
| FND-327 | manifest与export目标必须普通单链接文件 | artifact policy | 保障 | 高 | symlink/hardlink使审计对象与安装对象不同 | link负例 |
| FND-328 | internal runtime peer使用精确0.5.0 | manifests/policy | 保障 | 高 | 依赖所有权隐藏或运行时解析另一代 | packed manifest检查 |
| FND-329 | workspace协议仅用于内部dev/build | manifest policy + packed check | 开发运维 | 中 | 发布tar无法由外部npm安装 | tgz manifest无workspace |
| FND-330 | tar path必须canonical且留在package根 | tar inspector | 保障 | 高 | 安装路径逃逸/跨平台解释差异 | absolute/`..`/backslash负例 |
| FND-331 | tar拒绝duplicate member | tar inspector | 保障 | 高 | 不同解包器选择不同内容 | duplicate负例 |
| FND-332 | tar拒绝link和special file | tar inspector | 保障 | 高 | 安装对象可指向包外或设备 | symlink/hardlink/device负例 |
| FND-333 | tar拒绝意外源码/未发布文件 | expected inventory | 保障 | 中 | 内部测试/Secret/维护脚本泄露 | member allowlist |
| FND-334 | 临时空消费者离线同时安装8个tgz | package smoke | 开发运维 | 高 | 只在workspace link下成功的包被发布 | npm offline/ignore-scripts install |
| FND-335 | 离线消费者解析每个静态/代码export | smoke import/resolve | 开发运维 | 高 | 部分深入口在真实package缺失 | 全exports遍历 |
| FND-336 | release tool BuildIdentity复用五字段合同 | `sarmg_release::BuildIdentity` | 核心 | 高 | Python工具与Rust/TS release identity分叉 | round-trip/fixture |
| FND-337 | release manifest记录exact path/mode/size/hash | release.py | 保障 | 高 | 下载树无法证明完整和无额外文件 | build/parse/verify测试 |
| FND-338 | release path拒绝absolute/parent/backslash/NUL/超长 | path validator | 保障 | 高 | verifier读取树外或跨平台歧义 | 攻击fixture |
| FND-339 | release tree只允许普通单链接文件 | lstat/no-follow policy | 保障 | 高 | link/special file绕过hash对象 | link/device负例 |
| FND-340 | no-follow fd hash并复核inode/size/time/path | safe hashing | 保障 | 高 | TOCTOU替换绕过release验证 | race replacement/growth测试 |
| FND-341 | file/tree/count/manifest/size都有硬上限 | release policy constants | 保障 | 高 | 恶意树消耗无界CPU/内存/IO | limit与fail-fast调用次数测试 |
| FND-342 | tool tar固定mtime=0/uid/gid/name/mode/order | asset builder | 开发运维 | 中 | 相同源码无法复核字节一致性 | 双构建SHA相同 |
| FND-343 | release output必须安全、空、非root目录 | `prepare_output` | 保障 | 高 | 覆盖源码/宽目录或混入陈旧资产 | broad/nonempty/link目录负例 |
| FND-344 | release要求工作树无tracked/untracked变化 | `verify_source` | 保障 | 高 | tag资产包含未提交或本地文件 | porcelain必须空 |
| FND-345 | tag必须精确`v0.5.0`且唯一指向HEAD | `verify_source` | 保障 | 高 | 资产版本/source/tag不能建立一一关系 | wrong/missing/multiple tag负例 |
| FND-346 | state contract先生成再hash绑定release identity | asset builder顺序 | 保障 | 高 | identity引用错误状态合同 | hash与Schema验证 |
| FND-347 | build inventory记录工具链与两个lock hash | `inventory` | 开发运维 | 中 | 事故时无法重构构建输入 | JSON内容与hash测试 |
| FND-348 | inventory枚举6个Rust和4个npm组件 | cargo metadata + package discover | 开发运维 | 中 | 发布组件缺失/多余不易发现 | component集合检查 |
| FND-349 | `SHA256SUMS`覆盖全部非自身artifact | `checksums` | 保障 | 中 | 下载后只能信托管平台声明 | asset集合与sha256sum复核 |
| FND-350 | release-tree manifest置于artifacts外且不自描述 | builder布局 | 保障 | 高 | manifest递归hash或遗漏边界 | exact tree测试 |
| FND-351 | action只允许完整SHA allowlist | workflow policy | 保障 | 高 | mutable action tag被供应链替换 | valid/invalid fixture |
| FND-352 | runner固定ubuntu-24.04且job timeout有界 | workflow policy | 保障 | 中 | runner语义漂移或CI无限挂起 | floating/missing/oversize负例 |
| FND-353 | 顶层权限空，普通job只读contents | workflow policy | 保障 | 高 | PR/push代码获得不必要写权限 | write permission负例 |
| FND-354 | checkout不持久化credential | workflow policy | 保障 | 高 | 后续脚本/依赖可窃取Git token | missing/true负例 |
| FND-355 | setup-node固定26.7.0且不check-latest | workflow policy | 保障 | 中 | CI解析不同Node patch | floating/check-latest负例 |
| FND-356 | workflow拒绝YAML anchor/alias/merge及伪造action位置 | textual policy/tests | 保障 | 高 | 权限/action检查被YAML结构技巧绕过 | 专用fixtures |
| FND-357 | 唯一写权限只在tag-only release job | workflow policy | 保障 | 高 | 普通CI可创建/覆盖Release | 文件/job/trigger三重负例 |
| FND-358 | Cargo/pnpm lock提交并使用locked/frozen | lockfiles、CI命令 | 保障 | 中 | 同一commit每次解析不同依赖 | clean checkout验证 |

## 13. 测试、消费者与文档保障

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-360 | Rust执行fmt/check/clippy/test/doc完整门禁 | CI/README/operations | 开发运维 | 中 | target、lint、测试或rustdoc回归进入消费者 | locked/all-target/all-feature/-Dwarnings |
| FND-361 | Web执行typecheck、package unit与tar smoke分层 | pnpm scripts/package tool | 开发运维 | 中 | 源码通过但dist/tar失败 | clean workspace + offline smoke |
| FND-362 | Python policy/release/package有正反单元测试 | `tools/tests` | 开发运维 | 高 | fail-closed verifier只是未经证明的假设 | unittest discovery全部通过 |
| FND-363 | 管理员auth有策略/攻击/authority/cookie/CSRF测试 | `sarmg-admin-auth` tests | 保障 | 高 | 各产品依赖的认证下限可无声退化 | 每种typed error和边界 |
| FND-364 | admin-web有 login/logout/restore/401 的受控 Promise 竞态测试 | package tests | 保障 | 高 | 正常单请求通过但真实UI竞态失效 | controlled Promise/fetch交错；React client-switch 仍由消费者组件门禁验证 |
| FND-365 | contracts用跨语言共享fixture | contracts tests/crate include | 保障 | 高 | Rust/TS/Schema理解不同当前合同 | valid/invalid全量执行 |
| FND-366 | design token测试source/dist/value/scope | package test/scripts | 开发运维 | 中 | CSS发布漂移只能在视觉回归发现 | CSS静态与effective值测试 |
| FND-367 | consumer matrix有机器Schema与policy | `consumers/*.json` | 开发运维 | 中 | 无法知道谁采用、是否真实通过 | exact keys、known set、status约束 |
| FND-368 | `passing`必须有完整last_verified_commit | matrix policy | 保障 | 中 | 本地/过期结果被当发布证据 | null/短SHA负例 |
| FND-369 | `not-integrated`证据必须为空 | matrix policy | 保障 | 低 | 状态与组件/版本自相矛盾 | adopted/packages/verified负例 |
| FND-370 | package列表只记直接采用组件且unique | matrix Schema/policy | 开发运维 | 低 | 传递依赖冒充覆盖，影响评估失真 | duplicate/unknown负例 |
| FND-371 | 中文文档限定五类并与源码同步 | README/docs结构 | 开发运维 | 中 | 新成员误用安全primitive或依赖陈旧示例 | 链接/API/版本/命令抽查 |
| FND-372 | 功能台账逐项记录删除后果与验证边界 | 本文件 | 开发运维 | 中 | 删除共享能力时无法评估多仓库影响 | PR评审要求唯一ID同步 |

## 14. 明确排除项（同样属于功能边界）

| ID | 不进入 Foundation 的能力 | 当前归属/实现锚点 | 分类 | 复杂度 | 若强行加入的后果 | 验证边界 |
|---|---|---|---|---|---|---|
| FND-380 | 管理员用户表和账号生命周期 | 各产品DB/CLI/API | 核心 | 高 | 基础库拥有业务身份与删除/禁用语义 | Foundation无DB/route；产品集成测试 |
| FND-381 | Session表、TTL、并发上限、撤销/version | 各产品auth persistence | 核心 | 高 | 不同威胁模型被一个中央实现锁死 | 只共享token/contract；产品Session测试 |
| FND-382 | Cookie名称、Domain/Path/Secure/HttpOnly/SameSite | 各产品HTTP adapter | 核心 | 高 | 代理/部署差异被错误统一 | 产品Set-Cookie测试 |
| FND-383 | 登录限流、未知用户等成本和审计 | 各产品 | 保障 | 高 | 共享库无法掌握IP/account/body/容量边界 | 产品攻击/容量测试 |
| FND-384 | 设备、Agent、API key、媒体token等数据面身份 | 各产品协议 | 核心 | 高 | “仅管理员角色”被误解成删除业务credential | admin wire与数据面合同分离测试 |
| FND-385 | Axum/router middleware、body/rejection/request-ID注入 | 各Server | 核心 | 高 | 路由与日志策略被最低共同实现覆盖 | 真实router响应集成测试 |
| FND-386 | 产品配置/Secret loader | 各Server `config/`/env | 核心 | 高 | 环境变量、权限、Secret backend和fail-closed规则混淆 | 产品启动配置负例 |
| FND-387 | 路径no-follow/openat2/owner/mode | 各产品资源层 | 保障 | 高 | 通用弱封装引入TOCTOU/跨平台漏洞 | 产品fd相对/篡改测试 |
| FND-388 | 业务SQLite DDL、transaction与writer | 各产品 | 核心 | 高 | 基础库了解业务状态并阻碍独立演进 | Foundation只校验identity/baseline |
| FND-389 | migration与非当前Schema reader | `sarmg-upgrade`精确edge | 核心 | 高 | runtime携带历史分支并扩大权限面 | Foundation源码无migration SQL/reader |
| FND-390 | backup/restore journal与crash recovery | 升级工具/产品adapter | 核心 | 高 | 资源组合/Secret/原子替换语义被错误泛化 | 共享只提供manifest contract |
| FND-391 | 自动HTTP retry和mutation幂等 | 每个业务调用方 | 核心 | 高 | 通用层重复未知副作用 | http-client fetch一次；产品operation测试 |
| FND-392 | 文件/媒体stream transport | 产品client | 核心 | 高 | JSON body预算/parse不适用且占内存 | http-client只处理有界JSON |
| FND-393 | Web路由、页面、品牌、业务store | 各`clients/web` | 核心 | 高 | 产品被同一UI发布周期和信息架构耦合 | admin-web只提供auth/request/build primitive |
| FND-394 | 浏览器Session持久化 | 明确不实现 | 保障 | 高 | token长期暴露并改变重载/跨tab安全语义 | 无local/sessionStorage/IndexedDB源码 |
| FND-395 | UI组件库与字体 | 各产品 | 核心 | 中 | 表面统一扩大bundle和视觉耦合 | design package只含primitive CSS/TS |
| FND-396 | Dufs React/Vite迁移 | Dufs原生ES modules | 核心 | 高 | 重写成熟嵌入前端而无业务收益 | Dufs只共享管理员后端/wire合同 |
| FND-397 | 产品release目录/mode/self-binding规则 | 各产品release verifier | 保障 | 高 | 通用verifier成为更强产品边界的上限 | Foundation verifier后继续产品验证 |
| FND-398 | telemetry exporter/runtime | 各产品 | 核心 | 高 | 生命周期、隐私、字段和出口策略被中央化 | Foundation仅有CI/release审计 |
| FND-399 | Server安装、systemd、reverse proxy和运行配置 | 各产品`deploy/`/`config/` | 核心 | 高 | 无daemon仓库误拥有部署状态 | Foundation无deploy/config；消费者运维验证 |

## 15. 组件依赖与责任图

```text
sarmg-admin-auth ───────────────┐
                               ├─> sarmg-contracts
sarmg-error ────────────────────┤
sarmg-schema-identity ──────────┘
       └─> sarmg-sqlite

@sarmg/contracts ──> @sarmg/http-client ──┐
                                          ├─> @sarmg/admin-web
React/Vite peers ─────────────────────────┘

@sarmg/design-tokens（独立可选）
sarmg-server-target（只由Server binary直接采用）
```

依赖箭头不转移产品责任。例如 `sarmg-contracts` 依赖 admin-auth 只是复用 canonical username/token validator，
并不让合同 crate拥有密码数据库；`admin-web` 依赖 http-client 也不让它拥有 Server Cookie。

## 16. 关键取舍矩阵

| 选择 | 得到的收益 | 明确付出的成本 | 何时重新评审 |
|---|---|---|---|
| build-time而非中央service | 生产故障域独立、断网运行 | 每个产品都要显式升级重建 | 只有出现不可编入产品的真实共同能力 |
| current-only exact合同 | 漂移立即失败、边界可证明 | 破坏性变更需同步所有消费者 | 不用宽松兼容替代；历史转换进升级仓库 |
| 单一admin角色 | 授权面、Schema、UI和审计最小 | 不提供只读/操作员管理账户 | 有两个以上产品的真实分权需求和完整威胁模型时 |
| 精确Argon2 policy | 启动/登录成本和状态唯一 | 参数升级必须离线重建/转换 | 安全基线变化时发布新current版本 |
| 严格Origin/Host/Sec-Fetch-Site | 代理歧义和CSRF fail closed | 非浏览器脚本不能伪装管理页面 | 另建明确机器API，不加header fallback |
| 只支持AMD64 GNU/Linux Server | 部署、CI、ELF和运行假设一致 | 不提供ARM/musl Server | 补齐全产品等价构建/部署/安全矩阵后 |
| Dufs保留原生ES modules | 避免无收益重写，维持单binary模型 | 前端框架不是字面一致 | Dufs业务重构本身证明React收益时 |
| exact React/Vite版本 | 四个管理Web构建可复核 | 工具链升级需锁步 | 独立大问题验证所有消费者后 |
| 纯Schema算法+SQLx adapter | rusqlite/SQLx共享且无native link冲突 | 产品仍写少量driver映射 | 新driver出现时添加adapter而非复制算法 |
| no automatic retry | 不重复未知副作用 | 产品必须实现幂等/operation策略 | 仅在业务层有明确可重试操作时 |
| bounded buffered JSON | API简单且防无界内存 | 不适合大文件/媒体stream | 使用产品专用stream transport |
| scoped reset | 浏览器基线共享且不污染宿主 | 每个App需加scope attribute | 不改为全局reset |
| Git rev/release tgz消费 | 无需公共registry且来源不可变 | rev/URL与lock更新更显式 | 若采用可信registry provenance再评审 |

## 17. 删除或替换一项功能的完成定义

删除任一“核心/保障”项前，必须提供：受影响消费者与调用点；当前替代；安全/资源/竞态负例的等价证明；
持久状态与发布资产影响；不可变版本策略；每个产品的验证和回退计划。删除“建议保留/可选”项也必须先从
consumer matrix 和真实源码确认无人依赖。

执行删除时同步移除 member、dependency、export、Schema、fixture、test、lock、package/release inventory、
CI、文档和消费者调用，不留下 alias。若改变持久格式，在线产品直接只接受新当前格式；只有明确的稳定
source/target状态才在独立 `sarmg-upgrade` 创建转换。

## 18. 当前版本整体交付定义

`0.5.0` 只有在以下条件全部成立时才完成：22个crate和8个package身份一致；六个crate的Cargo package均
自带审核过的Apache-2.0文本；Rust/TS/Schema/fixture同构；
管理员唯一角色和认证策略被所有Server采用；非AMD64 Server编译失败而客户端平台不受误限；非Dufs Web
使用精确React/Vite基线；Dufs例外有文档和测试；SQLite current identity严格；真实tgz离线安装；release
tree可复核；workflow最小权限；消费者改用不可变来源并独立验证；中文文档准确；无兼容分支；每个大问题
独立Git提交并推送。
