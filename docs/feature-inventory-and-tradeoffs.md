<!-- 当前单体架构：xcss 是一个 Linux AMD64 GNU Rust crate 和一个 @xcss/web npm 包。能力表描述内部模块；不表示独立软件包可以单独编译或发布。 -->
# xcss 完整功能与取舍清单

## 0. 读法、分类和验收规则

本清单是开发人员判断“xcss 当前究竟实现了什么、删除后会发生什么、什么明确不属于它”的权威
边界台账，不是产品宣传页。每项都有唯一 ID、实现锚点、分类、复杂度、删除后果和最低验证/边界。修改
公开行为时，源码、测试、Schema/fixture、package/release、消费者和本表必须在同一大问题中同步。

分类只有五种：

- **核心**：定义 xcss 身份、公开合同或不可替代的跨项目语义；删除会改变项目性质或直接破坏消费者。
- **保障**：控制认证、安全、完整性、并发、资源或供应链风险；通常不增加业务页面，但不可轻率删除。
- **可选**：消费者按需采用，不应成为隐式依赖；删除仍需先确认实际消费者。
- **建议保留**：产品理论上可自行实现，但共享能显著降低漂移和重复事故。
- **开发运维**：构建、测试、发布、审计、文档、追踪与事故响应能力。

复杂度按“重新设计 + 实现 + 正反测试 + 所有消费者迁移与发布验证”的总成本评为低/中/高，而不是代码
行数。认证、跨语言、持久状态、竞态、release-tree 和多仓库变更通常为高。

## 1. 项目身份、架构和仅支持当前格式边界

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-001 | 纯构建期共享层，不运行中央 xcss service | 根 README、Cargo/npm 工作区、状态合同无资源 | 核心 | 高 | 引入生产网络依赖、中央故障域和所有产品锁步发布 | 消费者制品断网运行；无监听/daemon/config |
| FND-002 | 1 个 Rust crate 与 1 个 npm 软件包的内部模块共同发布 | 根 Cargo/npm 清单、src/web 内部模块 | 核心 | 中 | 小产品被迫引入 SQLx、React 或无关依赖，编译/审计面扩大 | 单体内部无软件包依赖；外部依赖树抽查 |
| FND-003 | 全组件当前清单版本 `1.0.0` | Cargo 软件包 version、npm 软件包 version、`xcss_policy.py` | 保障 | 中 | 同一发行内无法确定可组合组件，lock 和支持矩阵失真 | repository 规则精确一致性检查 |
| FND-004 | 只提供唯一当前 API/合同/算法 | crate/package public API、严格的运行时校验、文档 | 核心 | 高 | 兼容分支和测试矩阵持续膨胀，产品边界不可证明 | 不存在别名、dual reader/write、已弃用的导出入口 |
| FND-006 | xcss 不拥有用户、会话、业务 DB、文件树或进程 | 所有 crate/package API 范围 | 核心 | 高 | 共享库变成特权平台，产品无法独立运行和发布 | API/依赖审计；状态合同 resources 为空 |
| FND-007 | 生产不依赖 GitHub/npm/xcss 在线可用 | 消费者 pin、编译/打包模型 | 保障 | 高 | registry/GitHub 故障会让已部署产品不可用 | 构建后断网启动与核心功能验证 |
| FND-008 | 最强产品规则不得被通用辅助函数削弱 | 接入流程、消费者集成测试 | 保障 | 高 | Sunshine TLS、产品路径/release verifier 等边界被最低共同实现替换 | 产品先/后置加强验证保留，攻击负例不减少 |
| FND-009 | Rust 全工作区 `unsafe_code=deny` | `[workspace.lints.rust]` | 保障 | 中 | 新 unsafe 可绕过内存安全假设并扩大全仓审计 | all-target/all-feature check 与 lint |
| FND-010 | Clippy 禁止 `dbg!` 与 `todo!` | `[workspace.lints.clippy]` | 开发运维 | 低 | 临时诊断或未实现路径进入发布 crate | clippy `-D warnings` |
| FND-011 | Rust edition 2024、MSRV/toolchain 1.99 | 工作区、`rust-toolchain.toml` | 开发运维 | 中 | 各 crate 编译语义和依赖解析漂移 | 固定工具链检查、测试与文档生成 |
| FND-012 | Web 单包 Node 26.7.0、pnpm 10.34.6、TS 7.0.2 | `.node-version`、root/package 清单、lock | 开发运维 | 中 | 本地/CI/package 构建结果不一致 | 规则 + 按锁文件安装 + 软件包冒烟验证 |
| FND-013 | Apache-2.0 SPDX 元数据与审核文本一致；根 LICENSE 的 SHA-256 固定，唯一 crate 携带普通单链接审核过的 LICENSE，Cargo 软件包清单必须实际分发它，npm tgz也携带许可证 | 根 Cargo/npm 清单、`LICENSE`、根 `LICENSE`、`xcss_policy.py`、`check-rust-package-licenses.py` | 保障 | 中 | `cargo vendor` 展平 Git 依赖后丢失许可证，第三方 notices 只能失败或错误借用消费者通用文本，发布来源和使用权不可审计 | 根摘要、链接/字节负例、单体 crate 的 `cargo package --list`、npm tar inventory、Xczs notices E2E |
| FND-014 | xcss 无 `config/`、`deploy/`、`clients/` | 仓库布局 | 核心 | 低 | 容易误认为存在在线服务或产品 UI | 目录与状态合同审查 |
| FND-015 | 单个软件包的内部前端源码位于 `web/`，产物位于根 `dist/` | monorepo 布局 | 核心 | 低 | 发布依赖与可运行客户端职责混淆 | 软件包导出s 与消费者 import 验证 |

## 2. 管理员身份与密码：`xcss::admin_auth`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-020 | 管理员身份只允许一个规范的 ASCII username 合同 | `require_canonical_administrator_username` | 核心 | 高 | 各产品对同一管理身份产生不同 key、比较、会话与审计语义 | Rust/TS/Schema 共用正反测试夹具 |
| FND-021 | 登录候选只执行 `trim_ascii` + ASCII 小写 | `normalize_administrator_username` | 核心 | 中 | 产品可能执行 Unicode case folding 或其他改写，形成别名与限流绕过 | `" Admin.Ops " -> "admin.ops"`；Unicode/control 拒绝 |
| FND-022 | 登录候选必须是 1..64 可打印 ASCII 字节 | normalize 入口、contracts Rust/TS/Schema | 保障 | 中 | 无界或控制字符身份会在 Argon2/账户查询前进入日志、内存与限流表 | empty/1/64/65/control/DEL/Unicode 边界 |
| FND-023 | 规范的 username 长度固定 3..64 字节 | `ADMINISTRATOR_USERNAME_MIN_BYTES/MAX_BYTES` | 保障 | 低 | 过短身份易混淆；无上限会放大 DB、日志、UI 和索引成本 | 2/3/64/65 字节 |
| FND-024 | 规范的字符只允许小写字母、数字、`.`、`_`、`-` | username 验证器、TS regex、JSON Schema | 保障 | 中 | 空格、引号、路径/邮箱符号会在 SQL、URL、日志或跨语言中产生歧义 | 大写、`+`、`@`、空格、Unicode 负例 |
| FND-025 | 规范的首尾必须为 ASCII 字母或数字 | username 验证器、Schema pattern | 保障 | 低 | 首尾分隔符在复制、展示和配置中不易辨认且易被裁剪 | `.admin`、`admin_`、`-admin` 负例 |
| FND-026 | 管理身份明确不具有邮箱或 DNS domain 语义 | allowed set 不含 `@`，无 domain 解析器 | 核心 | 中 | 重新加入邮箱会引入 local/domain、大小写和地址生命周期规则，并使当前各产品字段再次分叉 | `admin@example.test` 拒绝；代码/Schema 无 email 字段 |
| FND-027 | 规范的允许相邻中间分隔符且不折叠点/横线/下划线 | username 验证器的字节精确比较 | 可选 | 低 | 若删除会缩小当前可用名称；若擅自折叠会把不同 username 合并 | `admin..ops`、`admin__ops` 正例；保持逐字节一致的 |
| FND-028 | 持久 username 与会话必须已规范的，xcss 不修复存量值 | `require_canonical_*` 与产品启动验证约定 | 保障 | 高 | 启动时静默改写身份会破坏会话、审计和外键关联 | persisted uppercase/space 值启动失败且零写入 |
| FND-029 | 密码长度 12～1024 字节 | `validate_password`、常量 | 核心 | 中 | 过短降低安全；无上限可放大 Argon2 资源消耗 | 11/12/1024/1025 字节 |
| FND-030 | 密码禁止 ASCII 控制字符 | `validate_password` | 保障 | 中 | 换行/NUL 等造成配置、日志、通信格式边界混淆 | 每类控制字符与普通 Unicode 测试 |
| FND-031 | 密码长度按 UTF-8 字节而非字符数 | `password.len()` | 核心 | 中 | Rust/其他语言对多字节密码预算分叉 | 多字节边界测试夹具 |
| FND-032 | 新散列只使用 Argon2id | `current_argon2` | 核心 | 高 | 产品散列算法不同，安全与运维合同碎片化 | PHC algorithm 精确检查 |
| FND-033 | Argon2 PHC version 只接受 v19 | `Version::V0x13`、规则 verifier | 核心 | 高 | verifier 接受其他语义版本，当前合同不唯一 | version 负例 |
| FND-034 | Argon2 memory 固定 19456 KiB | `ARGON2_MEMORY_KIB` | 核心 | 高 | 登录成本与容量规划跨产品漂移 | 参数减少/增加均拒绝 |
| FND-035 | Argon2 iterations 固定 2 | `ARGON2_ITERATIONS` | 核心 | 高 | 成本预算和当前 hash 身份不一致 | `t` 参数负例 |
| FND-036 | Argon2 parallelism 固定 1 | `ARGON2_PARALLELISM` | 核心 | 高 | CPU 并行成本随实现变化 | `p` 参数负例 |
| FND-037 | Argon2 salt 固定 16-byte fresh random | `SaltString::generate`、规则 verifier | 保障 | 高 | salt 重用/长度漂移降低或模糊安全合同 | 两次 hash 不同；非 16-byte拒绝 |
| FND-038 | Argon2 output 固定 32 字节 | `ARGON2_OUTPUT_BYTES` | 核心 | 高 | digest 长度不同却被误认为当前 hash | output length负例 |
| FND-039 | `hash_password` 先验证密码明文规则 | `hash_password` | 保障 | 中 | 非法密码仍被持久化为“合法 PHC” | 短/控制字符不生成 hash |
| FND-040 | `verify_password` 对非法密码明文直接 false | `verify_password` | 保障 | 中 | 登录路径对无界/非法输入仍执行昂贵 verifier | 输入边界和 false 语义测试 |
| FND-041 | PHC 必须能规范的往返一致性 | `hash.to_string() == encoded` | 保障 | 高 | 多种等价编码扩大持久状态和绕过检测面 | 非规范的编码拒绝 |
| FND-042 | 参数不同的有效 Argon2id hash也拒绝 | `password_hash_uses_current_policy` | 核心 | 高 | 在线 runtime 形成多规则回退 | weak/alternate 规则负例 |
| FND-043 | 可在无密码明文时检查持久 hash 当前性 | `require_current_password_hash` | 保障 | 高 | 服务只能在某管理员下次登录时才发现状态不合规 | 产品启动遍历所有管理员并 fail fast |
| FND-044 | xcss 不实现“成功登录后顺便升级 hash” | API 明确无此入口 | 核心 | 高 | 在线服务携带历史转换和双 verifier | API 面/依赖扫描；转换归独立流程 |

## 3. 令牌、Cookie、同源与 CSRF：`xcss::admin_auth`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-050 | Session/CSRF 令牌使用 32-byte OS 随机源 | `random_token`、`getrandom::fill` | 保障 | 高 | 令牌可预测、跨产品熵不同 | 长度/随机源失败路径；无回退 |
| FND-051 | 令牌使用 URL 安全的 Base64无填充 | `URL_SAFE_NO_PAD` | 核心 | 中 | Cookie/header/JSON 表示不一致 | 编码字符集与无 `=` 测试 |
| FND-052 | 当前令牌字符串恰好 43 字节 | `SESSION_TOKEN_ENCODED_BYTES`、`is_token_shape` | 核心 | 中 | 任意 raw 令牌或另一编码代进入会话 | 42/43/44 边界 |
| FND-053 | 令牌必须解码后再规范的 re-encode相同 | `is_token_shape` | 保障 | 高 | 非规范的末尾 bits 或等价表示被接受 | 非规范最后字符负例 |
| FND-054 | 令牌摘要固定 SHA-256 32 字节 | `token_hash`、`TOKEN_HASH_BYTES` | 核心 | 高 | 产品会话表摘要算法/长度分叉 | golden digest 与长度测试 |
| FND-055 | 提供小写 hex 令牌 hash | `token_hash_hex` | 建议保留 | 低 | 配置型产品重复 hex 实现易漂移 | 64 位小写 hex |
| FND-056 | 令牌比较先验证 shape 与 expected hash长度 | `token_matches_hash` | 保障 | 高 | 任意字符串摘要可能被当当前会话 | non规范令牌/31/33-byte digest负例 |
| FND-057 | 令牌 digest 比较使用 constant time | `subtle::ConstantTimeEq` | 保障 | 高 | 普通比较暴露时序侧信道 | code review + match正反测试 |
| FND-058 | Cookie 名称仅允许字母数字/下划线/连字符 | `parse_cookie_value` | 保障 | 中 | 调用方用歧义名称解析 raw Cookie | 非法 name负例 |
| FND-059 | Cookie 只返回唯一非空同名值 | `parse_cookie_value` | 保障 | 高 | 重复 cookie fixation/选择顺序歧义 | 空值/重复值/空格分隔负例 |
| FND-060 | 框架必须自行拒绝重复 Cookie field line | `parse_cookie_value` doc contract | 保障 | 高 | 辅助函数只见被框架合并的一行而漏掉歧义 | 产品集成重复 header测试 |
| FND-061 | 安全 header 辅助函数要求恰好一个 field-line value | `require_single_security_header_value` | 保障 | 高 | 框架挑第一/最后值造成 request smuggling语义 | missing/2行/3行具有类型约束的 error |
| FND-062 | 安全 header 只接受 nonempty visible ASCII | 同上 | 保障 | 中 | 控制字符/空值进入 URL、日志或比较 | 空、space、控制字符、non-UTF8负例 |
| FND-063 | 安全 header 拒绝逗号连接值 | 同上 | 保障 | 高 | 多值被代理合并后伪装成单值 | comma负例 |
| FND-064 | 同源校验显式区分 Production HTTPS 与回环地址 HTTP dev | `AdministratorOriginMode` | 核心 | 高 | 私网明文或环境猜测成为生产回退 | 两模式 scheme/host矩阵 |
| FND-065 | Production Origin 协议方案必须精确 `https` | `mode.scheme()` | 保障 | 高 | 管理凭据可经明文 HTTP | http production负例 |
| FND-066 | Development Origin 协议方案必须精确 `http` | `LoopbackDevelopmentHttp` | 核心 | 中 | 模式语义不唯一并掩盖代理错误 | https dev负例 |
| FND-067 | HTTP dev双方 host 必须是 localhost/127.0.0.0/8/::1 | `NormalizedHost::is_loopback` | 保障 | 高 | 私网/公网 HTTP 被误当开发例外 | 非回环地址 DNS/IP负例 |
| FND-068 | Origin、Host、Sec-Fetch-Site 都必须出现一次 | `require_administrator_same_origin` | 保障 | 高 | 缺失 header被静默接受，浏览器 CSRF 边界消失 | 三类 missing/duplicate负例 |
| FND-069 | `Sec-Fetch-Site` 必须精确 `same-origin` | 同上 | 保障 | 高 | cross-site/none/same-site 请求绕过 | 值枚举负例 |
| FND-070 | Origin 必须有且只有 `scheme://authority` | `split_once` + `parse_authority` | 保障 | 高 | path/query/fragment/userinfo等被 URL 解析器宽松接受 | 各 URL 部件攻击负例 |
| FND-071 | Host 支持 DNS、规范的 IPv4、bracketed IPv6 | authority 解析器 | 核心 | 高 | 各产品/IP版本对同源理解不同 | DNS/v4/v6正例 |
| FND-072 | DNS host比较时小写，拒绝首尾点/非法 label | `parse_unbracketed_host` | 保障 | 高 | 等价或无效 host 产生不一致 authority | case正例、尾点/非法字符负例 |
| FND-073 | IPv4/IPv6 必须采用规范的文本 | authority 解析器、`to_string` | 保障 | 高 | 多文本表示绕过 equality/log分析 | 非规范的 IP负例 |
| FND-074 | 端口 1..65535、无前导零，缺失时用协议方案默认 | `parse_port`、`default_port` | 保障 | 高 | `:0443`、0、溢出或默认端口理解不一 | 端口边界矩阵 |
| FND-075 | Origin authority 与有效 Host authority精确相等 | `OriginHostMismatch` | 保障 | 高 | cookie/CSRF 发往被混淆 host/port | 协议方案默认端口与显式端口测试 |
| FND-076 | 调用方必须把 HTTP/2 URI authority加入有效 Host值集合 | API doc、消费者适配器 | 保障 | 高 | `Host` 与 `:authority` 冲突时框架静默选择 | authority-only正例、二者冲突负例 |
| FND-077 | 不解析 `X-Forwarded-*` 或信任代理回退 | API 明确排除 | 核心 | 高 | 未经产品 trust boundary 的客户端可伪造外部来源 | API 面扫描；产品先形成权威 Host |
| FND-078 | CSRF header必须唯一、visible且为规范令牌 | `require_single_csrf_token` | 保障 | 高 | 多值/任意令牌进入比较 | missing/duplicate/comma/shape负例 |
| FND-079 | CSRF 与会话 digest常量时间比较 | `require_csrf_token_matches_hash` | 保障 | 高 | 修改操作防护变成普通字符串比较 | match/mismatch/错误长度测试 |
| FND-080 | crate不决定 Cookie 名/flags/TTL/Session table | public API边界 | 核心 | 高 | 不同部署和威胁模型被错误锁死或中央化 | 产品拥有 Secure/HttpOnly/SameSite/撤销测试 |

## 4. 管理员与错误通信格式 contract：`xcss::contracts` / `@xcss/web/contracts` / `xcss::error`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-090 | 三个管理员认证 path 是跨 Rust/TS常量 | `ADMIN_*_PATH`、`ADMIN_AUTH_PATHS` | 核心 | 高 | 产品 route/client产生双事实源 | 恰好 `/api/v1/auth/login|session|logout` |
| FND-091 | `AdministratorLoginRequest` 精确的两字段，Rust 读写均先验证 | Rust custom serialize/deserialize、TS 运行时校验、Schema | 核心 | 高 | 隐藏字段/多角色/设备字段进入管理登录，或手工 struct 输出非法通信格式 | missing/unknown/type 测试夹具；非法 public value 序列化失败 |
| FND-092 | 登录 username 候选 1..64 可打印 ASCII 字节 | Rust custom deserialize、TS 运行时校验、Schema | 保障 | 中 | 无界/Unicode/control body 在 Argon2/admission 前进入服务 | empty/64/65/control/DEL/Unicode 负例 |
| FND-093 | 登录 password候选 1..1024 码点且无控制字符 | 同上 | 保障 | 中 | 通信格式先无界；与服务端字节规则角色混淆 | empty/1025/control负例；服务端再按字节校验 |
| FND-094 | 管理角色枚举只有 `admin` | `AdministratorRole::Admin`、常量/Schema const | 核心 | 高 | 多角色扩展Schema、API、UI和授权状态空间 | viewer/operator/其他值拒绝 |
| FND-095 | 会话恰好五字段 | `AdministratorSession`、运行时校验、Schema | 核心 | 高 | 产品返回不同会话形状，Web 认证分叉 | 精确的 keys 测试夹具 |
| FND-096 | `authenticated` 必须为 literal true | custom deserialize/serialize、TS 运行时校验 | 核心 | 中 | “未认证会话对象”进入已认证流程 | false/null/missing负例 |
| FND-097 | `user_id` 是有界ASCII identifier | contracts 验证器 | 保障 | 中 | 任意展示文本/控制字符进入路由和日志 | 1..128、字符集边界 |
| FND-098 | 会话 username 必须已是规范管理员用户名 | admin-auth复用、TS regex、Schema | 核心 | 高 | 服务端与 Web 展示/比较不同身份或把登录候选误当持久值 | Rust/TS/Schema 同一测试夹具；uppercase/`@`拒绝 |
| FND-099 | 会话 `csrf_token` 必须当前43字符令牌 | admin-auth复用、TS 运行时校验、Schema | 保障 | 高 | Web持有无法被服务端当前CSRF验证的值 | 规范的末尾bits负例 |
| FND-100 | Rust 会话 constructor自动固定 `role=admin` | `AdministratorSession::new` | 保障 | 中 | 产品手写role或false authenticated | serialize验证和constructor测试 |
| FND-101 | `ErrorCode` 1..128 字节、小写字母开头 | `xcss::error::ErrorCode`、TS 运行时校验 | 核心 | 中 | UI按自由文本分支，日志/指标字段无界 | 首字符/长度/Unicode负例 |
| FND-102 | ErrorCode只含小写字母、数字、`.`、`_`、`-` | ErrorCode 验证器 | 保障 | 低 | 控制字符/空格进入日志、指标和client dispatch | 字符集测试夹具 |
| FND-103 | `RequestId` 1..128 字节有界ASCII identifier | `RequestId`、TS `isRequestId` | 保障 | 中 | header/body/log correlation可注入或无界 | 空/129/control/Unicode负例 |
| FND-104 | 错误响应结构拒绝未知 fields | Rust `deny_unknown_fields`、TS allowed keys、Schema | 核心 | 高 | 服务和client对错误含义静默分叉 | unknown/missing 测试夹具 |
| FND-105 | 错误响应结构固定 code/message/retryable | `ErrorEnvelope` | 核心 | 中 | client退回解析展示文案或猜可重试性 | required字段正反测试 |
| FND-106 | `request_id` 缺失与存在有效值区分，显式null拒绝 | custom deserialize、guard/Schema | 保障 | 中 | null/非法ID被当缺失，关联证据失真 | missing/null/invalid 测试夹具 |
| FND-107 | `details` 只能是对象，空对象序列化省略 | `Map<String,Value>`、guard/Schema | 保障 | 中 | raw任意JSON/诊断泄露，shape不稳定 | array/scalar负例；空值 serialization |
| FND-108 | message只用于展示，machine分支使用code/status | 类型doc与http-client | 核心 | 中 | 文案调整破坏业务逻辑和国际化 | 消费者不按message分支审查 |
| FND-109 | 9个常用HTTP status与code映射 | `HttpStatus` | 建议保留 | 低 | 各产品重复映射并可能把所有失败变500 | 400/401/403/404/409/422/429/500/503测试 |
| FND-110 | 默认仅429/503可重试 | `default_retryable` | 保障 | 中 | client可能重放有副作用请求或永不重试临时失败 | status映射测试；产品可显式覆盖 |
| FND-111 | Rust error类型可构造、parse、display与serde | impl集合 | 建议保留 | 中 | 产品复制有界identifier实现 | 往返一致性与invalid serde测试 |
| FND-112 | Rust/TS/Error Schema共用相同测试夹具 | contracts 测试夹具 + Rust include | 保障 | 高 | 错误路径跨语言漂移只在生产出现 | valid/invalid在两端全部执行 |

## 5. 状态、发行与备份合同

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-120 | 状态合同通信格式 version固定1 | `STATE_CONTRACT_VERSION`、Schema/guard | 核心 | 高 | 工具无法判断状态描述语义 | 其他version拒绝 |
| FND-121 | State 精确的 fields且未知拒绝 | Rust serde、TS 运行时校验、Schema | 核心 | 高 | 产品遗漏/扩展被另一端静默忽略 | missing/unknown 测试夹具 |
| FND-122 | application/version为有界identifier | contract validators | 核心 | 中 | 产品身份不能稳定进入release/backup | 字符与长度边界 |
| FND-123 | source_revision恰好40位小写hex | custom deserialize、TS 运行时校验、Schema | 保障 | 中 | 发行无法绑定一个精确Git对象 | 39/41/uppercase/nonhex负例 |
| FND-124 | State schema字段必须出现，可为null | custom required option、运行时校验精确的 keys | 核心 | 高 | “无DB”与“忘记声明”混淆 | missing/null/object三态测试夹具 |
| FND-125 | State schema 修订号为非负安全整数 | custom 安全整数、运行时校验 | 保障 | 高 | JS静默失真或负版本进入工具 | MAX_SAFE/+1/fraction/negative负例 |
| FND-126 | State schema SHA为64位小写hex | 验证器 | 核心 | 中 | Schema身份不可复核 | hash边界测试夹具 |
| FND-127 | maintenance lock为唯一identifier数组 | unique 验证器 | 保障 | 高 | 重复/歧义lock导致工具顺序错误 | duplicate/invalid lock负例 |
| FND-128 | State resource kind只有5种当前值 | `StateResourceKind`、TS union、Schema | 核心 | 高 | 工具对未知持久资源做错误处理 | 未知 kind拒绝 |
| FND-129 | State resource恰好name/kind/required | struct/guard/Schema | 核心 | 中 | 必需资源语义被遗漏或附带未知字段 | exact-field 测试夹具 |
| FND-130 | External requirement描述kind/kid/algorithm/envelope_version | contract struct/guard | 核心 | 高 | backup/restore不知道缺哪个外部Secret合同 | identifier与positive 安全整数测试 |
| FND-131 | Companion contract绑定name/version/platform/SHA | contract struct/guard | 核心 | 高 | native companion可与服务端状态不匹配 | 精确的 fields/hash 测试夹具 |
| FND-132 | 发行身份恰好五字段 | `ReleaseIdentity`、Schema/guard | 核心 | 高 | 制品身份出现多套格式或升级字段泄入 | 精确的 key 测试夹具 |
| FND-133 | 发行 product/version/target为identifier | validators | 核心 | 中 | 发布工具对路径/平台身份解释不同 | invalid identifier负例 |
| FND-134 | 发行绑定完整源码修订号 | `source_revision` | 保障 | 高 | 同版本制品不能追溯源码 | 40位SHA与tag/HEAD复核 |
| FND-135 | 发行用state_contract_sha256绑定状态合同 | identity字段、asset builder | 保障 | 高 | 制品与状态/恢复合同可被独立替换 | build顺序和hash drift负例 |
| FND-137 | 备份清单通信格式 version固定2 | `BACKUP_MANIFEST_VERSION` | 核心 | 高 | 离线工具无法确定备份语义 | 其他version拒绝 |
| FND-138 | Backup 精确的 fields且未知拒绝 | Rust/TS/Schema | 核心 | 高 | 恢复端忽略重要字段或接受模糊数据 | shared 测试夹具 |
| FND-139 | Backup 结构身份复用完整四字段SchemaIdentity | type 别名与Schema | 核心 | 高 | 备份身份和DB身份算法复制漂移 | null/object及exact-current由产品加强 |
| FND-140 | Backup created_at为非负safe epoch seconds | 安全整数验证器 | 保障 | 中 | JS失真或负时间破坏审计 | max/+1/negative/fraction负例 |
| FND-141 | Backup resources至少一项 | custom deserialize/validate、运行时校验 | 保障 | 高 | 空归档被标记成功备份 | 空值 array负例 |
| FND-142 | Backup resource path非空但不声称规范的 | `validate_non_empty_path` | 核心 | 中 | 通用层若猜路径规则会弱化产品不跟随符号链接的/布局 | 空值拒绝；产品继续验证canonical/path root |
| FND-143 | Backup 字节非负safe、files正安全整数 | validators | 保障 | 高 | 资源计数溢出/空文件集合被当有效 | 0 files、un安全整数负例 |
| FND-144 | Backup每个资源有SHA-256 | `BackupResource` | 保障 | 高 | 清单无法绑定实际资源内容 | hash格式 + 产品实际hash复核 |
| FND-145 | Backup external requirement额外绑定Secret材料SHA | `BackupExternalRequirement` | 保障 | 高 | key ID相同但材料不同仍被误用 | sha/algorithm/envelope 精确的测试 |
| FND-146 | Rust State/Release/Backup直接include TS 软件包测试夹具 | `include_str!`测试 | 保障 | 高 | 两语言各自测试但语义仍漂移 | 同一valid/invalid集合 |
| FND-147 | JSON Schema与测试夹具作为公开软件包导出 | contracts manifest/copy script | 建议保留 | 中 | 非Rust/TS工具复制合同或深层导入 | 发行归档导出入口解析 |

## 6. Schema 身份：`xcss::schema_identity`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-150 | Schema算法与数据库driver解耦 | 模块算法不调用SQLx/rusqlite，单体包统一持有服务端依赖 | 核心 | 高 | SQLx/rusqlite native link冲突或复制算法 | 依赖树、双driver消费者 |
| FND-151 | 规范的 `product_metadata` DDL恰好五列 | `PRODUCT_METADATA_DDL` | 核心 | 高 | 产品元数据字段/约束分叉 | DDL 测试夹具与产品migration对比 |
| FND-152 | singleton INTEGER PK NOT NULL CHECK=1 | DDL/column 验证器 | 保障 | 高 | 多身份row或无唯一锚点 | DDL/PRAGMA/row负例 |
| FND-153 | application/application_version TEXT NOT NULL | DDL/column 验证器 | 核心 | 中 | SQLite动态类型让身份变为null/其他存储类别 | column和runtime typeof检查 |
| FND-154 | schema_revision INTEGER NOT NULL | DDL/column 验证器 | 核心 | 中 | 修订号形态漂移 | column/storage/negative测试 |
| FND-155 | schema_sha256 TEXT NOT NULL | DDL/column 验证器 | 核心 | 中 | hash以blob/null等形式持久化 | column/storage/hash测试 |
| FND-156 | 五列顺序/cid/type/notnull/pk/default精确验证 | `validate_product_metadata_columns` | 保障 | 高 | 同名但不同shape的表被接受 | 每个列属性漂移负例 |
| FND-157 | DDL比较只忽略ASCII空白与字母case | `validate_product_metadata_ddl` | 核心 | 高 | 语义不同DDL被过度normalize为相同 | check/default/列变化负例 |
| FND-158 | 元数据必须恰好一row | `schema_identity_from_metadata_rows` | 保障 | 高 | 任意选择0/多row身份 | 0/2 row 具有类型约束的 error |
| FND-159 | singleton值必须精确1 | `ProductMetadataRow::to_schema_identity` | 保障 | 中 | 错误row被当产品身份 | 0/2负例 |
| FND-160 | schema 修订号不能为负 | i64→u64 checked conversion | 保障 | 中 | SQLite负数绕过通信格式非负规则 | -1负例 |
| FND-161 | identity四分量都参与精确的当前比较 | `SchemaIdentity::require_exact` | 核心 | 高 | 只比hash/revision会接纳其他产品/版本DB | 每字段mismatch 具有类型约束的 error |
| FND-162 | identifier 1..128 ASCII安全字符 | identity 验证器 | 保障 | 中 | 产品/version进入日志/manifest时无界或注入 | 字符/长度测试 |
| FND-163 | schema SHA必须64位小写 hex | `validate_schema_sha256` | 核心 | 中 | 非规范的或错误hash进入身份 | upper/length/nonhex负例 |
| FND-164 | 结构指纹算法版本常量为1 | `SCHEMA_FINGERPRINT_ALGORITHM_VERSION` | 核心 | 高 | 不同实现无法声明字节算法 | golden vector版本 |
| FND-165 | 查询排除 `sqlite_*` 内部对象 | `SQLITE_SCHEMA_ROWS_QUERY` +函数二次拒绝 | 保障 | 高 | SQLite版本/内部对象改变产品hash | excluded row负例 |
| FND-166 | 查询排除 `product_metadata` 自身 | query +函数二次拒绝 | 核心 | 高 | 元数据声明hash与hash输入自引用 | included 元数据负例 |
| FND-167 | 行按 type/name/table的BINARY顺序 | query ORDER BY +函数检查 | 核心 | 高 | adapter/locale/返回顺序改变hash | 乱序负例 |
| FND-168 | 重复 Schema object key拒绝 | `DuplicateSchemaObject` | 保障 | 高 | 重复输入被无声hash入且适配器差异 | 重复负例 |
| FND-169 | 每row四字段分别加入u64 大端序字节长度 | `schema_fingerprint` framing | 核心 | 高 | 简单拼接产生边界碰撞或跨语言分叉 | golden framing vectors |
| FND-170 | SQL按原始UTF-8 字节进入hash | `digest.update(bytes)` | 核心 | 高 | formatter/语义归一化把真实DDL drift合并 | 空白/Unicode SQL差异vector |
| FND-171 | 声明结构指纹与实际结构指纹双重比较 | `verify_fingerprint`/`verify_current_schema` | 保障 | 高 | 只改元数据即可伪装当前DB | declared/actual mismatch负例 |
| FND-172 | 发布可复核golden vectors JSON | crate 测试夹具常量 | 开发运维 | 中 | 新driver/语言无法独立证明同算法 | 测试夹具 parse与expected hash |
| FND-173 | 具有类型约束的 error指出row/column/identity/order/hash字段 | `Error`/`IdentityField` | 建议保留 | 中 | doctor只能解析字符串，现场定位困难 | error variant单元测试 |

## 7. SQLx SQLite 基线：`xcss::sqlite`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-180 | `open_existing` 明确拒绝缺失数据库 | path `try_exists` + create=false | 保障 | 高 | 拼错路径会被SQLite悄悄创建空库 | 缺失具有类型约束的 error；产品先做不跟随符号链接的 |
| FND-181 | `create_if_missing` 是单独显式入口 | create=true API | 核心 | 高 | 初始化意图与正常启动混淆 | 缺失创建、existing打开测试 |
| FND-182 | pool max connections必须大于0 | `PoolOptions::validate` | 保障 | 低 | 无效pool在运行时产生难懂错误 | 0负例 |
| FND-183 | min connections不能超过max | 同上 | 保障 | 低 | 配置自相矛盾 | 边界负例 |
| FND-184 | acquire 超时必须非零，默认10秒 | options/常量 | 保障 | 中 | 无限/立即等待语义跨产品漂移 | zero/default/custom测试 |
| FND-185 | 每连接强制foreign_keys=ON | `SqliteConnectOptions` | 保障 | 高 | 新连接可写悬空引用 | 多连接PRAGMA与FK violation |
| FND-186 | 每连接强制WAL | journal mode | 建议保留 | 高 | 读写并行、checkpoint和备份语义分叉 | PRAGMA实际值 |
| FND-187 | 每连接强制synchronous=FULL | synchronous mode | 保障 | 高 | 断电持久窗口扩大 | PRAGMA实际值 |
| FND-188 | busy 超时固定5秒 | `BUSY_TIMEOUT` | 保障 | 中 | 锁冲突随机立即失败或无限等待 | PRAGMA/锁竞争测试 |
| FND-189 | 完整性摘要 check必须唯一返回`ok` | `integrity_check` | 保障 | 高 | 损坏DB继续运行或进入备份 | 多诊断/损坏测试夹具 |
| FND-190 | FK check收集table/row/parent/index | `ForeignKeyViolation` | 保障 | 高 | 只知道失败无法定位或完全漏检 | 具体violation断言 |
| FND-191 | TRUNCATE checkpoint busy是失败 | `CheckpointBusy` | 保障 | 高 | 仍有读取器时只复制main file造成不完整备份 | busy 读取器测试 |
| FND-192 | checkpoint frame不完整也是失败 | `CheckpointIncomplete` | 保障 | 高 | 部分WAL被误当安全checkpoint | tuple 验证器负例 |
| FND-193 | schema rows可从pool/connection/transaction executor读取 | generic `Executor<Sqlite>` | 建议保留 | 中 | 产品重复适配器并绕开事务一致视图 | 三类调用/编译测试 |
| FND-194 | SQLx 适配器复用纯结构指纹算法 | `fingerprint_rows` | 核心 | 高 | SQLx与离线rusqlite工具hash分叉 | shared golden/current schema测试 |
| FND-195 | 读取元数据前同时校验DDL、列与存储类别 | `validate_metadata_table`/`typeof` query | 保障 | 高 | SQLite动态类型或近似表冒充当前合同 | DDL/PRAGMA/typeof负例 |
| FND-196 | `read_schema_identity` 先验证实际hash才返回身份 | function顺序 | 保障 | 高 | 调用方误信未绑定真实DDL的元数据 | drifted DDL测试 |
| FND-197 | pool convenience仍执行相同当前验证 | `read_pool_*`/`require_pool_*` | 建议保留 | 低 | 产品为便利绕过connection级验证 | 封装集成测试 |
| FND-198 | 不含业务DDL、migration、路径权限、实例锁、backup | crate public API | 核心 | 高 | 通用层越权修改产品状态或给出错误安全保证 | API面审计；产品生命周期测试 |
| FND-199 | `xcss::platform_db` 提供当前平台元数据单例的事务内初始化、严格读取与运行形态验证；产品仍拥有业务初始数据 | `initialize_current_platform_metadata`、`require_current_platform_metadata` | 核心 | 高 | 产品复制平台 SQL 或只建表不写记录，当前平台身份无法闭环 | 新库初始化后立即验证；空行、重复行、错误 Profile/代际拒绝 |

## 8. 服务端架构门禁：`xcss::server_target`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-200 | 服务端唯一target为`x86_64-unknown-linux-gnu` | `SERVER_TARGET_TRIPLE` | 核心 | 高 | 各产品对“支持平台”有不同声明和制品 | 常量、发行身份、文档一致 |
| FND-201 | 目标架构必须为 x86_64 | compile-time cfg | 保障 | 高 | ARM 服务端可能未经等价验证被发布 | aarch64 编译失败 |
| FND-202 | 目标操作系统必须为 Linux | compile-time cfg | 保障 | 高 | Windows/macOS 服务端绕过Linux文件/服务假设 | cross-OS 编译失败 |
| FND-203 | target env必须GNU/glibc | compile-time cfg | 保障 | 高 | musl制品与动态加载/部署合同不符 | musl 编译失败 |
| FND-204 | pointer width必须64 | compile-time cfg | 保障 | 中 | 32位资源/ABI边界未验证 | cfg gate |
| FND-205 | 非目标在编译期直接`compile_error!` | crate root | 保障 | 高 | 仅依赖CI约定，开发者可本地误构建 | 负值 target job/手动编译 |
| FND-206 | 提供human architecture常量`amd64` | `SERVER_ARCHITECTURE` | 建议保留 | 低 | release/install脚本各自命名架构 | 常量测试 |
| FND-207 | runtime/release 元数据可精确检查target | `require_server_target` | 保障 | 中 | 清单宣称与编译target漂移 | accepted 规范的、所有近似值拒绝 |
| FND-208 | 客户端/Client不依赖该crate | crate文档与消费者Cargo边界 | 核心 | 高 | xsoc/移动/桌面客户端被误限制为AMD64 Linux | 依赖 tree + 各客户端平台构建 |
| FND-209 | xcss整体只允许Linux AMD64 GNU编译及发布 | 发行 target `x86_64-unknown-linux-gnu` | 核心 | 中 | source 软件包和服务端平台概念混淆 | 发行身份检查 |

## 9. JSON HTTP 客户端：`@xcss/web/http-client`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-220 | `requestJson<T>` 是唯一请求入口 | 软件包导出 | 核心 | 中 | 产品fork多个不同安全辅助函数 | export/tarball测试 |
| FND-221 | 无浏览器location时必须显式绝对baseUrl | `resolveSameOriginUrl` | 保障 | 中 | Node测试/SSR错误解析相对URL | no-location负例 |
| FND-222 | URL只允许HTTP/HTTPS | `parseHttpUrl` | 保障 | 高 | data/file/javascript等协议方案进入fetch | 协议方案负例且fetch未调用 |
| FND-223 | URL拒绝控制字符和空值 | `parseHttpUrl` | 保障 | 中 | 日志/解析混淆 | control/empty负例 |
| FND-224 | URL拒绝用户信息 | username/password检查 | 保障 | 高 | 凭据嵌入URL并可能泄漏 | 用户信息负例 |
| FND-225 | target 来源与base 来源精确相等 | 来源比较 | 保障 | 高 | cookie/CSRF请求发往第三方 | scheme/host/port变化负例 |
| FND-226 | 重定向固定`error`且不允许调用方放宽 | request validation/fetch init | 保障 | 高 | 30x绕过前置同源检查 | override负例、capture init |
| FND-227 | 凭据默认`same-origin` | fetch init | 核心 | 中 | 会话 cookie不发送或策略各产品漂移 | fetch capture |
| FND-228 | 默认Accept application/json，保留其他header | Headers logic | 建议保留 | 低 | 内容协商不稳定或调用方 header丢失 | header merge测试 |
| FND-229 | method必须string并统一大写判断安全性 | method 验证 | 保障 | 中 | 非法method或大小写绕过CSRF分类 | method type/case测试 |
| FND-230 | 只对非安全 HTTP 方法注入CSRF | `isUnsafeMethod` | 保障 | 高 | 令牌泄入safe request或修改操作漏令牌 | GET/HEAD/OPTIONS/TRACE vs POST/PUT/PATCH/DELETE |
| FND-231 | CSRF必须非空且无CR/LF | 令牌验证 | 保障 | 高 | header injection或空防护 | empty/newline负例 |
| FND-232 | 超时默认10秒、范围1..120000ms 安全整数 | constants/`validateBudget` | 保障 | 中 | 请求无限挂起或配置溢出 | 上下界/fraction/NaN负例 |
| FND-233 | 调用方 AbortSignal与超时 first-wins合并 | local AbortController/source | 保障 | 高 | 错误分类竞态随机，listener泄漏 | caller-first/timeout-first交错测试 |
| FND-234 | finally清timer和调用方 listener | `finally` | 保障 | 中 | 长期页面积累资源泄漏 | fake timer/listener测试 |
| FND-235 | network/timeout/caller abort有不同具有类型约束的 code | catch/localError | 核心 | 高 | UI误报或错误重试修改操作 | 三类失败测试 |
| FND-236 | 成功响应默认2MiB、最大64MiB | constants/budget 验证 | 保障 | 高 | `response.json()`无界占内存 | exact/+1和配置上限测试 |
| FND-237 | 错误正文独立固定64KiB上限 | `MAX_ERROR_RESPONSE_BYTES` | 保障 | 高 | 大型代理错误页耗尽内存 | oversize error测试 |
| FND-238 | 先拒绝过大声明Content-Length并取消body | `readBoundedText` | 保障 | 高 | 已知超限仍读取全部内容 | declared size/cancel测试 |
| FND-239 | 再流式累计实际Uint8Array 字节 | reader/chunks | 保障 | 高 | chunked/错误Content-Length绕过预算 | chunked exact/+1测试 |
| FND-240 | 超限时读取器 cancel失败不覆盖权威size error | nested try/catch | 保障 | 中 | transport cleanup异常改变错误分类 | cancel throw测试 |
| FND-241 | UTF-8使用fatal 解码 | `TextDecoder(...,{fatal:true})` | 保障 | 中 | replacement char使JSON/文本静默变化 | 畸形 UTF-8测试 |
| FND-242 | success/error只接受JSON或`+json`Content-Type | `isJsonContentType` | 保障 | 中 | HTML/文本错误页进入业务状态或UI | MIME参数、html/plain负例 |
| FND-243 | HEAD/204/205/Content-Length 0返回undefined | 空值 response分支 | 核心 | 中 | logout/no-content被误解析为invalid JSON | 各空响应测试 |
| FND-244 | 非2xx严格解析共享错误响应结构 | `responseError` + 运行时校验 | 核心 | 高 | client按任意upstream body分支 | valid/invalid envelope测试 |
| FND-245 | 非法error body不回显raw HTML/Secret | 通用安全错误 | 保障 | 高 | 内部诊断或Secret泄入UI/log | sentinel secret不可见断言 |
| FND-246 | body request_id优先、header 回退且均严格过滤 | safeRequestId/responseError | 保障 | 中 | correlation ID注入或错误关联 | body/header优先级与非法值测试 |
| FND-247 | Retry-After支持safe整数秒并封顶24h | 解析器 | 建议保留 | 中 | 各UI等待提示溢出/理解不同 | 0/large/unsafe测试 |
| FND-248 | Retry-After日期只接受规范HTTP-date 往返一致性 | regex/Date/UTCString | 保障 | 中 | 宽松日期解析跨runtime不一致 | canonical/noncanonical日期测试 |
| FND-249 | 401 callback被等待完成 | response path | 保障 | 高 | 会话清理与路由竞态不可控 | async callback顺序测试 |
| FND-250 | 401 callback异常不覆盖权威API error | callback catch | 保障 | 高 | 本地cleanup错误吞掉服务端 401证据 | throwing callback测试 |
| FND-251 | 从不自动重试 | API无重试 loop | 核心 | 高 | 未知副作用修改操作被重复执行 | fetch调用次数负例 |
| FND-252 | 泛型T不声称运行时验证 | JSON parse cast + docs | 核心 | 中 | 调用方把静态类型当网络信任边界 | 产品运行时校验集成测试 |
| FND-253 | 不用于文件上传/下载或streaming媒体 | 软件包边界 | 核心 | 高 | JSON预算/parse破坏大对象业务 | API面与产品专用transport |

## 10. 管理员 Web：`@xcss/web/admin-web`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-260 | 统一精确Node/React/Vite/TS工具链常量 | `ADMIN_WEB_TOOLCHAIN` | 核心 | 高 | 四个React产品依赖版本和构建资源包行为漂移 | 每个值与manifest/toolchain测试 |
| FND-261 | 清单 assertion检查Node engine精确字符串 | `assertAdministratorWebToolchain` | 保障 | 中 | 产品声明宽松engine掩盖本地/CI差异 | 任何范围变化拒绝 |
| FND-262 | `.node-version`只接受精确值可带单个换行 | 同上 | 保障 | 低 | 工作站工具链与软件包 engine分叉 | exact/extra文本负例 |
| FND-263 | dependencies/dev/peer/optional中出现工具链包就必须精确值 | 清单遍历 | 保障 | 高 | 通过另一section藏caret/不同补丁 | 每section drift负例 |
| FND-264 | 不强制消费者使用pnpm | assertion只检查工具链，不查packageManager | 核心 | 中 | npm消费者被无意义迁移或双lock | xsos/xscs/xszs/xcos npm lock验证 |
| FND-265 | 浏览器默认baseUrl为当前页面来源根 | `resolveAdministratorBaseUrl` | 核心 | 中 | 各Web对相对认证路径理解不同 | browser location测试 |
| FND-266 | 浏览器显式baseUrl仍必须同来源 | base/当前页面来源比较 | 保障 | 高 | 管理Cookie/CSRF被发送到第三方 | 跨源负例 |
| FND-267 | baseUrl只允许HTTP(S)且无用户信息 | URL 验证 | 保障 | 高 | 非网络协议方案或嵌入凭据 | protocol/userinfo负例 |
| FND-268 | Node/nonbrowser必须显式baseUrl | runtime location 运行时校验 | 保障 | 中 | 测试环境隐式依赖全局location | no-location负例 |
| FND-269 | 业务path必须以`/api/v1/`开头 | `resolveApiPath` | 保障 | 高 | shared client被用来访问任意页面/origin | prefix/control/hash负例 |
| FND-270 | path解析后仍检查来源与pathname | URL revalidation | 保障 | 高 | `//host`、编码或URL解析技巧越界 | origin/prefix攻击负例 |
| FND-271 | 调用方不能自设`X-CSRF-Token` | send header ownership检查 | 保障 | 高 | stale/伪造令牌绕过当前会话所有权 | existing header负例 |
| FND-272 | body存在且无Content-Type时自动JSON | send header逻辑 | 建议保留 | 低 | 各产品遗漏JSON content type | capture测试；保留调用方明确值 |
| FND-273 | 凭据只能未设置或同源 | send init检查 | 保障 | 高 | 调用方改成include/omit破坏认证模型 | override负例 |
| FND-274 | 成功响应必须通过调用方运行时校验 | `send<T>` | 保障 | 高 | 任意JSON进入React状态 | false 运行时校验产生`invalid_response_shape` |
| FND-275 | Session/CSRF仅保存在closure内存 | `session`/`transportSession`局部变量 | 保障 | 高 | XSS后长期令牌残留、跨tab/重启语义扩大 | 源码扫描无storage；重载需恢复会话 |
| FND-276 | publish对所有订阅者同步通知并可unsubscribe | listener Set | 建议保留 | 中 | 产品认证 store实现分叉或泄漏listener | subscribe/unsubscribe测试 |
| FND-277 | 登录 request先经过共享候选运行时校验 | `isAdministratorLoginRequest` | 保障 | 中 | 无界/控制字符先发到服务端 | client拒绝且fetch未调用 |
| FND-278 | login/logout认证修改操作全局串行 | `authenticationMutationTail` | 保障 | 高 | Set-Cookie网络完成顺序与UI调用顺序冲突 | overlap顺序测试 |
| FND-279 | 每次login/logout/当前401推进generation | `authenticationGeneration` | 保障 | 高 | 较旧响应复活或清除较新会话 | delayed response竞态测试 |
| FND-280 | 登录开始立即清旧UI 会话 | `publish(null)` | 保障 | 中 | 凭据切换时旧授权页面继续操作 | transition测试 |
| FND-281 | `transportSession`只服务排队退出的CSRF | private snapshot | 保障 | 高 | 登录后立刻退出可能无法撤销服务端 Cookie，或出现双授权状态 | overlapping login/logout测试 |
| FND-282 | superseded 认证 operation返回具有类型约束的 client error | `auth_operation_superseded` | 核心 | 中 | UI无法区分当前失败与过期结果 | generation测试 |
| FND-283 | 并发恢复会话复用同一Promise | `restorePromise` | 保障 | 高 | 页面多组件启动造成重复会话轮换/请求 | fetch调用一次测试 |
| FND-284 | 恢复会话等待之前排队的认证修改操作 | `precedingMutations` | 保障 | 高 | 恢复会话与Set-Cookie login/logout乱序 | overlap测试 |
| FND-285 | 恢复会话仅在当前generation发布会话 | `requireCurrentOperation` | 保障 | 高 | 延迟恢复会话覆盖新login/logout | delayed 恢复会话测试 |
| FND-286 | invalid 会话 response清当前状态 | 恢复会话 catch code集合 | 保障 | 高 | 服务漂移后UI继续信任本地会话 | shape/content/json/size错误测试 |
| FND-287 | 401只在dispatch 会话仍为当前时invalidate | 认证 context比较 | 保障 | 高 | 旧请求延迟401登出新会话 | stale 401竞态测试 |
| FND-288 | 退出本地授权同步结束，服务端请求仍排队完成 | 退出流程 | 保障 | 高 | 用户点击退出后仍短暂可操作，或Cookie不撤销 | immediate state + network顺序测试 |
| FND-289 | 退出 finally清transport 会话 | finally | 保障 | 中 | 失败后私有CSRF快照继续存在 | network failure测试 |
| FND-290 | React 钩子提供四态discriminated union | `AdministratorSessionState` | 核心 | 中 | 页面以多个boolean组合出非法认证状态 | TS类型与render测试 |
| FND-291 | 钩子 mount自动恢复会话 | `useEffect` | 建议保留 | 中 | 产品各自遗漏会话恢复或闪烁逻辑 | lifecycle测试 |
| FND-292 | 钩子用active client ref阻止换client后的旧更新 | `activeClient` | 保障 | 高 | tenant/base/client切换后旧响应污染新页面 | client swap测试 |
| FND-293 | 钩子用state generation阻止旧Promise覆盖 | `stateGeneration` | 保障 | 高 | React异步login/restore竞态 | 交错执行测试 |
| FND-294 | 钩子把401/superseded归anonymous，其余恢复会话失败归error | catch分类 | 核心 | 中 | 网络故障被误显示为“未登录”或反之 | error分类测试 |
| FND-295 | Vite 辅助函数固定React plugin、dist与emptyOutDir | `createXcssReactViteConfig` | 开发运维 | 中 | 产品构建插件/输出和stale asset策略漂移 | config对象与产品build检查 |
| FND-296 | Vite 辅助函数只允许产品选择base | `XcssReactViteOptions` | 核心 | 中 | 共享配置越权控制产品业务构建资源包 | public API审查 |
| FND-297 | Xczs明确不采用React/Vite 辅助函数 | 项目级例外与root差异文档 | 核心 | 高 | 为表面统一重写成熟嵌入前端，扩大风险 | Xczs保持ES modules但共享认证合同 |

## 11. 设计原语：`@xcss/web/design-tokens`

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-300 | TS导出小型颜色/间距/圆角原语 | `src/index.ts` | 可选 | 低 | 程序化图表/样式复制magic value | declaration/export/test |
| FND-301 | light语义令牌与原语对齐 | `semanticTokens.light`、`tokens.css` | 建议保留 | 中 | 多Web同义背景/文字/action命名漂移 | source/CSS effective value测试 |
| FND-302 | dark只覆盖真正变化的语义值 | `semanticTokens.dark`、`tokens.dark.css` | 建议保留 | 中 | 暗色漏项或复制整套原语 | light+override effective测试 |
| FND-303 | CSS变量统一`--xcss-*` 命名空间 | 令牌 CSS | 保障 | 中 | 宿主页或产品变量冲突 | selector/property扫描 |
| FND-304 | reset只在`[data-xcss-scope]`内生效 | `reset.css` | 保障 | 高 | 全局污染Xczs/嵌入页/第三方内容 | 禁止无作用域全局selector测试 |
| FND-305 | box-sizing对作用域及后代统一 | 限定作用域的 reset | 建议保留 | 低 | 浏览器布局差异回到每个产品 | CSS source/dist测试 |
| FND-306 | 表单继承font且disabled cursor明确 | reset | 建议保留 | 低 | 控件视觉/交互基线漂移 | CSS规则测试 |
| FND-307 | heading/text wrap与hidden基线 | reset | 建议保留 | 低 | 长身份/错误溢出或hidden失效 | CSS规则测试 |
| FND-308 | `:focus-visible`提供清晰键盘焦点 | accessibility CSS | 保障 | 中 | 键盘用户无法定位焦点 | CSS + 产品键盘测试 |
| FND-309 | reduced-motion显著压缩动画/transition | media query | 保障 | 中 | 动态敏感用户无法使用 | media query/source-dist测试 |
| FND-310 | forced-colors使用系统Highlight | media query | 保障 | 中 | 高对比度模式焦点不可见 | CSS + 人工高对比度验证 |
| FND-311 | visually-hidden保留辅助技术文本 | utility class | 建议保留 | 低 | 产品重复易错的屏幕阅读器隐藏模式 | 精确的 CSS测试 |
| FND-312 | 清理后构建删除陈旧CSS/JS声明 | clean/copy scripts | 开发运维 | 中 | 已删令牌仍在发行归档可被深层使用 | stale artifact负例 |
| FND-313 | 不包含组件、品牌、字体、主题状态或CDN | 软件包边界 | 核心 | 高 | 所有Web被单一视觉发行 cadence耦合 | API/tar inventory/network扫描 |

## 12. 软件包、发布树与供应链

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-320 | 单个npm 软件包统一metadata/license/engine/repository | 清单 + 规则 | 开发运维 | 中 | 来源、许可、工具链不可审计 | 清单 check |
| FND-321 | 软件包只发布`dist` | `files:["dist"]` | 保障 | 中 | src/test/cache/Secret意外进入tar | tar inventory |
| FND-322 | 每个公开入口显式写入导出入口 | 软件包清单 | 核心 | 中 | 消费者深层导入私有布局或入口漏包 | import.meta.resolve全部导出入口 |
| FND-323 | contracts复制5份Schema与5份测试夹具 | copy-contract-data script | 核心 | 中 | 机器合同或跨语言测试数据缺失 | dist/tar 导出入口检查 |
| FND-324 | admin-web复制共享tsconfig公开入口 | copy-config script | 建议保留 | 中 | 产品严格的编译基线复制漂移 | 导出入口解析与内容测试 |
| FND-325 | design-token复制4份CSS入口 | copy-css script | 核心 | 中 | 软件包声明与实际CSS缺失 | source=dist/tar测试 |
| FND-326 | build前clean且拒绝linked dist | 软件包 artifact 规则 | 保障 | 高 | 陈旧输出或符号链接逃逸进入包 | linked/stale dist负例 |
| FND-327 | 清单与导出入口目标必须普通单链接文件 | artifact 规则 | 保障 | 高 | symlink/hardlink使审计对象与安装对象不同 | link负例 |
| FND-328 | 内部模块使用同包公开子路径 | manifests/policy | 保障 | 高 | 依赖所有权隐藏或运行时解析另一代 | packed 清单检查 |
| FND-329 | 内部软件包依赖与工作区协议被删除 | 清单规则 + packed check | 开发运维 | 中 | 发布tar无法由外部npm安装 | tgz 清单无工作区 |
| FND-330 | tar path必须规范的且留在软件包根 | tar inspector | 保障 | 高 | 安装路径逃逸/跨平台解释差异 | absolute/`..`/backslash负例 |
| FND-331 | tar拒绝重复 member | tar inspector | 保障 | 高 | 不同解包器选择不同内容 | 重复负例 |
| FND-332 | tar拒绝link和special file | tar inspector | 保障 | 高 | 安装对象可指向包外或设备 | symlink/hardlink/device负例 |
| FND-333 | tar拒绝意外源码/未发布文件 | expected inventory | 保障 | 中 | 内部测试/Secret/维护脚本泄露 | member allowlist |
| FND-334 | 临时空消费者安装唯一真实tgz | 软件包冒烟验证 | 开发运维 | 高 | 只在工作区 link下成功的包被发布 | npm offline/ignore-scripts install |
| FND-335 | 离线消费者解析每个静态/代码导出入口 | 冒烟验证 import/resolve | 开发运维 | 高 | 部分深入口在真实软件包缺失 | 全导出入口遍历 |
| FND-336 | 发行 tool BuildIdentity复用五字段合同 | `xcss_release::BuildIdentity` | 核心 | 高 | Python工具与Rust/TS 发行身份分叉 | 往返一致性/fixture |
| FND-337 | 发行清单记录精确的 path/mode/size/hash | release.py | 保障 | 高 | 下载树无法证明完整和无额外文件 | build/parse/verify测试 |
| FND-338 | 发行 path拒绝absolute/parent/backslash/NUL/超长 | path 验证器 | 保障 | 高 | verifier读取树外或跨平台歧义 | 攻击测试夹具 |
| FND-339 | 发行目录只允许普通单链接文件 | lstat/不跟随符号链接的规则 | 保障 | 高 | link/special file绕过hash对象 | link/device负例 |
| FND-340 | 不跟随符号链接的 fd hash并复核inode/size/time/path | safe hashing | 保障 | 高 | TOCTOU替换绕过发行验证 | race replacement/growth测试 |
| FND-341 | file/tree/count/manifest/size都有硬上限 | 发行规则 constants | 保障 | 高 | 恶意树消耗无界CPU/内存/IO | limit与fail-fast调用次数测试 |
| FND-342 | tool tar固定mtime=0/uid/gid/name/mode/order | asset builder | 开发运维 | 中 | 相同源码无法复核字节一致性 | 双构建SHA相同 |
| FND-343 | 发行 output必须安全、空、非root目录 | `prepare_output` | 保障 | 高 | 覆盖源码/宽目录或混入陈旧资产 | broad/nonempty/link目录负例 |
| FND-344 | 发行要求工作树无tracked/untracked变化 | `verify_source` | 保障 | 高 | 标签资产包含未提交或本地文件 | porcelain必须空 |
| FND-345 | 标签必须精确`v1.0.0`且唯一指向HEAD | `verify_source` | 保障 | 高 | 资产版本/source/tag不能建立一一关系 | wrong/missing/multiple 标签负例 |
| FND-346 | 状态合同先生成再hash绑定发行身份 | asset builder顺序 | 保障 | 高 | identity引用错误状态合同 | hash与Schema验证 |
| FND-347 | build inventory记录工具链与两个lock hash | `inventory` | 开发运维 | 中 | 事故时无法重构构建输入 | JSON内容与hash测试 |
| FND-348 | inventory枚举1个Rust和1个npm组件 | cargo 元数据 + 软件包 discover | 开发运维 | 中 | 发布组件缺失/多余不易发现 | component集合检查 |
| FND-349 | `SHA256SUMS`覆盖全部非自身artifact | `checksums` | 保障 | 中 | 下载后只能信托管平台声明 | asset集合与sha256sum复核 |
| FND-350 | release-tree 清单置于artifacts外且不自描述 | builder布局 | 保障 | 高 | 清单递归hash或遗漏边界 | 精确的 tree测试 |
| FND-351 | action只允许完整SHA allowlist | 工作流校验规则 | 保障 | 高 | mutable action 标签被供应链替换 | valid/invalid 测试夹具 |
| FND-352 | runner固定ubuntu-24.04且job 超时有界 | 工作流校验规则 | 保障 | 中 | runner语义漂移或CI无限挂起 | floating/missing/oversize负例 |
| FND-353 | 顶层权限空，普通job只读contents | 工作流校验规则 | 保障 | 高 | PR/push代码获得不必要写权限 | write permission负例 |
| FND-354 | 源码检出不持久化凭据 | 工作流校验规则 | 保障 | 高 | 后续脚本/依赖可窃取Git 令牌 | missing/true负例 |
| FND-355 | setup-node固定26.7.0且不check-latest | 工作流校验规则 | 保障 | 中 | CI解析不同Node 补丁 | floating/check-latest负例 |
| FND-356 | workflow拒绝YAML anchor/alias/merge及伪造action位置 | textual policy/tests | 保障 | 高 | 权限/action检查被YAML结构技巧绕过 | 专用测试夹具 |
| FND-357 | 唯一写权限只在tag-only 发行 job | 工作流校验规则 | 保障 | 高 | 普通CI可创建/覆盖发行 | 文件/job/trigger三重负例 |
| FND-358 | Cargo/pnpm lock提交并使用locked/frozen | lockfiles、CI命令 | 保障 | 中 | 同一commit每次解析不同依赖 | clean 源码检出验证 |

## 13. 测试、消费者与文档保障

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-360 | Rust执行fmt/check/clippy/test/doc完整门禁 | CI/README/operations | 开发运维 | 中 | target、lint、测试或rustdoc回归进入消费者 | locked/all-target/all-feature/-Dwarnings |
| FND-361 | Web执行typecheck、软件包 unit与tar 冒烟验证分层 | pnpm scripts/package tool | 开发运维 | 中 | 源码通过但dist/tar失败 | 干净工作区 + offline 冒烟验证 |
| FND-362 | Python policy/release/package有正反单元测试 | `tools/tests` | 开发运维 | 高 | fail-closed verifier只是未经证明的假设 | unittest discovery全部通过 |
| FND-363 | 管理员认证有策略/攻击/authority/cookie/CSRF测试 | `xcss::admin_auth` tests | 保障 | 高 | 各产品依赖的认证下限可无声退化 | 每种具有类型约束的 error和边界 |
| FND-364 | admin-web有 login/logout/restore/401 的受控 Promise 竞态测试 | 软件包 tests | 保障 | 高 | 正常单请求通过但真实UI竞态失效 | controlled Promise/fetch交错；React client-switch 仍由消费者组件门禁验证 |
| FND-365 | contracts用跨语言共享测试夹具 | contracts tests/crate include | 保障 | 高 | Rust/TS/Schema理解不同当前合同 | valid/invalid全量执行 |
| FND-366 | design 令牌测试source/dist/value/scope | 软件包 test/scripts | 开发运维 | 中 | CSS发布漂移只能在视觉回归发现 | CSS静态与effective值测试 |
| FND-371 | 中文文档限定五类并与源码同步 | README/docs结构 | 开发运维 | 中 | 新成员误用安全原语或依赖陈旧示例 | 链接/API/版本/命令抽查 |
| FND-372 | 功能台账逐项记录删除后果与验证边界 | 本文件 | 开发运维 | 中 | 删除共享能力时无法评估多仓库影响 | PR评审要求唯一ID同步 |

### 13.1 正式内嵌 Web 与共同构建

| ID | 当前功能/特性 | 实现/锚点 | 分类 | 复杂度 | 删除后的确定后果 | 最低验证/边界 |
|---|---|---|---|---|---|---|
| FND-400 | 正式 Web 字节内嵌且与服务端同一身份 | `xcss::web_assets::build`、ADR-0009 | 核心 | 高 | 可执行文件读取另一版本的 dist，发布身份不完整 | 编译资源快照、清单摘要、实际 binary 验收 |
| FND-401 | 清单自动生成并绑定路径/MIME/大小/SHA-256 | `EmbeddedAsset`、`verify_embedded` | 保障 | 高 | 人工 include 列表遗漏字体/许可证，外部清单可授权不同字节 | 排序、规范的 JSON、篡改与资源遗漏负例 |
| FND-402 | GET/HEAD/304、准确 MIME 与缓存统一 | `xcss::web_assets::response` | 核心 | 中 | 多产品处理条件请求与 HTML 缓存时分叉 | HEAD 空 body、ETag 列表/weak/*、HTML no-store |
| FND-403 | 明确开发目录与每次请求安全读取 | `DirectoryAssets` | 开发运维 | 高 | 前端改动需要反复编译 Rust或开发模式渗入正式包 | 热更新、Unix descriptor/不跟随符号链接的、链接/路径穿越负例 |
| FND-404 | 前端→Rust→实际资源的共同构建顺序 | `@xcss/web/web-toolchain/server`、`xcss-build-server` | 保障 | 高 | 打包旧 dist，编译资源与本次 Web 输出不一致 | 干净源码、锁定依赖、规范 target、执行 binary `web-assets` |
| FND-405 | capability 与构建声明接入门禁 | `xcss_conformance.policy` | 保障 | 中 | 使用新版本却继续维护产品私有资源实现 | 缺失 capability/declaration、目录逃逸、缺共同 runtime/build crate 负例 |

## 14. 明确排除项（同样属于功能边界）

| ID | 不进入 xcss 的能力 | 当前归属/实现锚点 | 分类 | 复杂度 | 若强行加入的后果 | 验证边界 |
|---|---|---|---|---|---|---|
| FND-380 | 管理员用户表和账号生命周期 | 各产品DB/CLI/API | 核心 | 高 | 基础库拥有业务身份与删除/禁用语义 | xcss无DB/route；产品集成测试 |
| FND-381 | 会话表、TTL、并发上限、撤销/version | 各产品认证 persistence | 核心 | 高 | 不同威胁模型被一个中央实现锁死 | 只共享token/contract；产品会话测试 |
| FND-382 | Cookie名称、Domain/Path/Secure/HttpOnly/SameSite | 各产品HTTP 适配器 | 核心 | 高 | 代理/部署差异被错误统一 | 产品Set-Cookie测试 |
| FND-383 | 登录限流、未知用户等成本和审计 | 各产品 | 保障 | 高 | 共享库无法掌握IP/account/body/容量边界 | 产品攻击/容量测试 |
| FND-384 | 设备、客户端、API key、媒体令牌等数据面身份 | 各产品协议 | 核心 | 高 | “仅管理员角色”被误解成删除业务凭据 | admin 通信格式与数据面合同分离测试 |
| FND-385 | Axum/router middleware、body/rejection/request-ID注入 | 各服务端 | 核心 | 高 | 路由与日志策略被最低共同实现覆盖 | 真实router响应集成测试 |
| FND-386 | 产品配置/Secret loader | 各服务端 `config/`/env | 核心 | 高 | 环境变量、权限、Secret backend和fail-closed规则混淆 | 产品启动配置负例 |
| FND-387 | 路径不跟随符号链接的/openat2/owner/mode | 各产品资源层 | 保障 | 高 | 通用弱封装引入TOCTOU/跨平台漏洞 | 产品fd相对/篡改测试 |
| FND-388 | 业务SQLite DDL、事务与writer | 各产品 | 核心 | 高 | 基础库了解业务状态并阻碍独立演进 | xcss只校验identity/baseline |
| FND-389 | migration与非当前Schema 读取器 | 当前维护边界不提供此能力 | 核心 | 高 | runtime携带历史分支并扩大权限面 | xcss源码无migration SQL/reader |
| FND-391 | 自动HTTP 重试和修改操作幂等 | 每个业务调用方 | 核心 | 高 | 通用层重复未知副作用 | http-client fetch一次；产品operation测试 |
| FND-392 | 文件/媒体流 transport | 产品client | 核心 | 高 | JSON body预算/parse不适用且占内存 | http-client只处理有界JSON |
| FND-393 | Web路由、页面、品牌、业务store | 各`web` | 核心 | 高 | 产品被同一UI发布周期和信息架构耦合 | admin-web只提供auth/request/build 原语 |
| FND-394 | 浏览器会话持久化 | 明确不实现 | 保障 | 高 | 令牌长期暴露并改变重载/跨tab安全语义 | 无local/sessionStorage/IndexedDB源码 |
| FND-395 | UI组件库与字体 | 各产品 | 核心 | 中 | 表面统一扩大构建资源包和视觉耦合 | design 软件包只含原语 CSS/TS |
| FND-396 | 产品 Web 运行形态选择 | 产品清单声明 React 或原生 ESM | 核心 | 高 | 重写成熟嵌入前端而无业务收益 | 消费者独立验收所选运行形态与业务页面 |
| FND-397 | 产品发行目录/mode/self-binding规则 | 各产品发行 verifier | 保障 | 高 | 通用verifier成为更强产品边界的上限 | xcss verifier后继续产品验证 |
| FND-398 | telemetry exporter/runtime | 各产品 | 核心 | 高 | 生命周期、隐私、字段和出口策略被中央化 | xcss仅有CI/release审计 |
| FND-399 | 服务端安装、systemd、反向代理和运行配置 | 各产品`deploy/`/`config/` | 核心 | 高 | 无守护进程仓库误拥有部署状态 | xcss无deploy/config；消费者运维验证 |

## 15. 组件依赖与责任图

```text
xcss::admin_auth ───────────────┐
                               ├─> xcss::contracts
xcss::error ────────────────────┤
xcss::schema_identity ──────────┘
       └─> xcss::sqlite

@xcss/web/contracts ──> @xcss/web/http-client ──┐
                                          ├─> @xcss/web/admin-web
React/Vite peers ─────────────────────────┘

@xcss/web/design-tokens（独立可选）
xcss::server_target（只由Server binary直接采用）
```

依赖箭头不转移产品责任。例如 `xcss::contracts` 依赖 admin-auth 只是复用规范的 username/token 验证器，
并不让合同 crate拥有密码数据库；`admin-web` 依赖 http-client 也不让它拥有服务端 Cookie。

## 16. 关键取舍矩阵

| 选择 | 得到的收益 | 明确付出的成本 | 何时重新评审 |
|---|---|---|---|
| 构建期而非中央service | 生产故障域独立、断网运行 | 每个产品都要显式升级重建 | 只有出现不可编入产品的真实共同能力 |
| 仅支持当前格式精确的合同 | 漂移立即失败、边界可证明 | 破坏性变更需同步所有消费者 | 不使用宽松兼容或历史格式转换 |
| 单一admin角色 | 授权面、Schema、UI和审计最小 | 不提供只读/操作员管理账户 | 有两个以上产品的真实分权需求和完整威胁模型时 |
| 精确Argon2 规则 | 启动/登录成本和状态唯一 | 参数变化需显式更新当前凭据 | 安全基线变化时发布新当前版本 |
| 严格Origin/Host/Sec-Fetch-Site | 代理歧义和CSRF fail closed | 非浏览器脚本不能伪装管理页面 | 另建明确机器API，不加header 回退 |
| 只支持AMD64 GNU/Linux 服务端 | 部署、CI、ELF和运行假设一致 | 不提供ARM/musl 服务端 | 补齐全产品等价构建/部署/安全矩阵后 |
| Xczs保留原生ES modules | 避免无收益重写，维持单binary模型 | 前端框架不是字面一致 | Xczs业务重构本身证明React收益时 |
| 精确的 React/Vite版本 | 四个管理Web构建可复核 | 工具链升级需锁步 | 独立大问题验证所有消费者后 |
| 纯Schema算法+SQLx 适配器 | rusqlite/SQLx共享且无native link冲突 | 产品仍写少量driver映射 | 新driver出现时添加适配器而非复制算法 |
| 不自动重试 | 不重复未知副作用 | 产品必须实现幂等/operation策略 | 仅在业务层有明确可重试操作时 |
| 有界缓冲的 JSON | API简单且防无界内存 | 不适合大文件/媒体流 | 使用产品专用流 transport |
| 限定作用域的 reset | 浏览器基线共享且不污染宿主 | 每个App需加作用域 attribute | 不改为全局reset |
| Git rev/release tgz消费 | 无需公共软件包注册中心且来源不可变 | rev/URL与lock更新更显式 | 若采用可信软件包注册中心来源记录再评审 |

## 17. 删除或替换一项功能的完成定义

删除任一“核心/保障”项前，必须提供：受影响消费者与调用点；当前替代；安全/资源/竞态负例的等价证明；
持久状态与发布资产影响；不可变版本策略；每个产品的验证和回退计划。删除“建议保留/可选”项也必须先从
实际产品源码、依赖图和调用点确认无人依赖。

## 18. 当前版本整体交付定义

`1.0.0`只有在以下条件全部成立时才完成：单个 crate 和单个 npm 软件包身份一致；唯一 Cargo 软件包与 npm tgz 均
自带审核过的Apache-2.0文本；Rust/TS/Schema/fixture同构；
管理员唯一角色和认证策略被所有服务端采用；非AMD64 服务端编译失败而客户端平台不受误限；非Xczs Web
使用精确React/Vite基线；Xczs例外有文档和测试；SQLite 当前身份严格；真实tgz离线安装；发行
tree可复核；workflow最小权限；消费者改用不可变来源并独立验证；中文文档准确；无兼容分支；每个大问题
独立Git提交并推送。
带 Web 的服务端还必须采用共同构建声明与 `xcss::web_assets`，真实二进制内嵌字节通过共同验收；
产品安装、状态、签名和部署校验继续有效。前端生产改动需要重建 binary，开发热更新通过显式开发模式。

## 当前工作树公共机制补充（尚未发布）

| ID | 实际实现 | 权威锚点与验证边界 |
|---|---|---|
| FND-300 | 分层具有类型约束的 JSON 配置、显式环境映射、叶字段来源与每层语义钩子 | `xcss::config`；结构/来源/秘密/输入预算测试；产品完成最终必需字段和外部前提检查 |
| FND-301 | 机器 CLI 错误、真实服务就绪状态身份核验、共同 HTTP 解析拒绝 | `xcss::server_cli`；单记录输出、HTTP parser/413/no-store、临时监听测试 |
| FND-302 | 同目录运行/维护/诊断写锁、持久维护门和通用维护描述符借用 | `xcss::state_file`；真实 flock、inode、pending、显式发行、持久维护门和真实owner/root权限测试 |
| FND-303 | 当前 WAL/journal 代的只读临时数据库校验副本 | `xcss::sqlite::validation_snapshot`；源字节不变、writer busy、query-only与clone 运行时校验测试；仅独立诊断进程，不声明备份 |
| FND-304 | 服务端结构化日志、公共事件模板、精确筛选和有界轮转 | `xcss::log`；UTC/脱敏/limits/query/真实rotation/tracing sink切换；GNU/Linux AMD64 原生验证与产品验收 |
| FND-305 | 既有静态管理员只读检查、只在首次初始化写当前账户文件 | `xcss::admin_static`；当前格式与准确configured IDs、不创建文件、不改持久字节 |

工作树能力不能借用不可变 v0.10.4 标签的发布证据。正式版本、完整修订号、锁闭包、package/license清单和产品发行物需在受控发布时同步验收。API、适用边界和公共模块的实际消费入口见 [配置、CLI、锁与日志](configuration-cli-logging.md)。
