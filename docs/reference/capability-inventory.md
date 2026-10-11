# 能力与实现索引

按模块查找现有能力、实现入口和测试范围。编号沿用原清单，供维护时定位；首次接入请读[任务指南](../README.md)。

## 项目身份、架构和仅支持当前格式边界

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-001 | 构建期共享层，不运行中央 xcss service | 根 README、唯一 Cargo/npm 清单、无资源的状态合同 | 产品制品断网运行；xcss 不独立监听或运行 daemon |
| FND-002 | 1 个 Rust crate 与 1 个 npm 软件包的内部模块共同发布 | 根 Cargo/npm 清单、`src/` 与 `web/` 内部模块 | 一个 crate、一个 tgz；模块选择不移除根 Rust 依赖；Web 外部 peer 按实际需求安装 |
| FND-003 | 全组件当前清单版本 `1.0.2` | Cargo 软件包 version、npm 软件包 version、`xcss_policy.py` | repository 规则精确一致性检查 |
| FND-004 | 只提供唯一当前 API/合同/算法 | crate/package public API、严格的运行时校验、文档 | 不存在别名、dual reader/write、已弃用的导出入口 |
| FND-006 | 公共认证、存储、运行时机制编入各产品，不拥有中央在线实例或产品业务数据 | `admin_core`、`admin_sqlite`、`server_runtime`、`platform_db` 等内部模块 | 产品拥有自己的管理员状态、业务 DB、文件树与进程；xcss 状态合同 resources 为空 |
| FND-007 | 生产不依赖 GitHub/npm/xcss 在线可用 | 消费者 pin、编译/打包模型 | 构建后断网启动与核心功能验证 |
| FND-008 | 最强产品规则不得被通用辅助函数削弱 | 接入流程、消费者集成测试 | 产品先/后置加强验证保留，攻击负例不减少 |
| FND-009 | 根 crate 默认 `unsafe_code=deny`，审计模块显式限定例外 | 根 `[lints.rust]`、模块级审计说明 | all-target/all-feature check、lint 与 unsafe 审计 |
| FND-010 | Clippy 禁止 `dbg!` 与 `todo!` | 根 `[lints.clippy]` | clippy `-D warnings` |
| FND-011 | Rust edition 2024、MSRV/toolchain 1.99 | 根 Cargo 清单、`rust-toolchain.toml` | 固定工具链检查、测试与文档生成 |
| FND-012 | Web 单包 Node 26.7.0、pnpm 10.34.6、TS 7.0.2 | `.node-version`、root/package 清单、lock | 规则 + 按锁文件安装 + 软件包冒烟验证 |
| FND-013 | Apache-2.0 SPDX 元数据与审核文本一致；根 LICENSE 的 SHA-256 固定，唯一 crate 携带普通单链接审核过的 LICENSE，Cargo 软件包清单必须实际分发它，npm tgz也携带许可证 | 根 Cargo/npm 清单、`LICENSE`、根 `LICENSE`、`xcss_policy.py`、`check-rust-package-licenses.py` | 根摘要、链接/字节负例、单体 crate 的 `cargo package --list`、npm tar inventory、Xczs notices E2E |
| FND-014 | xcss 无 `config/`、`deploy/`、`clients/` | 仓库布局 | 目录与状态合同审查 |
| FND-015 | 单个软件包的内部前端源码位于 `web/`，产物位于根 `dist/` | 单包源码与构建布局 | 软件包导出与消费者 import 验证 |

## 管理员身份与密码：`xcss::admin_auth`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-020 | 管理员身份只允许一个规范的 ASCII username 合同 | `require_canonical_administrator_username` | Rust/TS/Schema 共用正反测试夹具 |
| FND-021 | 登录候选只执行 `trim_ascii` + ASCII 小写 | `normalize_administrator_username` | `" Admin.Ops " -> "admin.ops"`；Unicode/control 拒绝 |
| FND-022 | 登录候选必须是 1..64 可打印 ASCII 字节 | normalize 入口、contracts Rust/TS/Schema | empty/1/64/65/control/DEL/Unicode 边界 |
| FND-023 | 规范的 username 长度固定 3..64 字节 | `ADMINISTRATOR_USERNAME_MIN_BYTES/MAX_BYTES` | 2/3/64/65 字节 |
| FND-024 | 规范的字符只允许小写字母、数字、`.`、`_`、`-` | username 验证器、TS regex、JSON Schema | 大写、`+`、`@`、空格、Unicode 负例 |
| FND-025 | 规范的首尾必须为 ASCII 字母或数字 | username 验证器、Schema pattern | `.admin`、`admin_`、`-admin` 负例 |
| FND-026 | 管理身份明确不具有邮箱或 DNS domain 语义 | allowed set 不含 `@`，无 domain 解析器 | `admin@example.test` 拒绝；代码/Schema 无 email 字段 |
| FND-027 | 规范的允许相邻中间分隔符且不折叠点/横线/下划线 | username 验证器的字节精确比较 | `admin..ops`、`admin__ops` 正例；保持逐字节一致的 |
| FND-028 | 持久 username 与会话必须已规范的，xcss 不修复存量值 | `require_canonical_*` 与产品启动验证约定 | persisted uppercase/space 值启动失败且零写入 |
| FND-029 | 密码长度 12～1024 字节 | `validate_password`、常量 | 11/12/1024/1025 字节 |
| FND-030 | 密码禁止 ASCII 控制字符 | `validate_password` | 每类控制字符与普通 Unicode 测试 |
| FND-031 | 密码长度按 UTF-8 字节而非字符数 | `password.len()` | 多字节边界测试夹具 |
| FND-032 | 新散列只使用 Argon2id | `current_argon2` | PHC algorithm 精确检查 |
| FND-033 | Argon2 PHC version 只接受 v19 | `Version::V0x13`、规则 verifier | version 负例 |
| FND-034 | Argon2 memory 固定 19456 KiB | `ARGON2_MEMORY_KIB` | 参数减少/增加均拒绝 |
| FND-035 | Argon2 iterations 固定 2 | `ARGON2_ITERATIONS` | `t` 参数负例 |
| FND-036 | Argon2 parallelism 固定 1 | `ARGON2_PARALLELISM` | `p` 参数负例 |
| FND-037 | Argon2 salt 固定 16-byte fresh random | `SaltString::generate`、规则 verifier | 两次 hash 不同；非 16-byte拒绝 |
| FND-038 | Argon2 output 固定 32 字节 | `ARGON2_OUTPUT_BYTES` | output length负例 |
| FND-039 | `hash_password` 先验证密码明文规则 | `hash_password` | 短/控制字符不生成 hash |
| FND-040 | `verify_password` 对非法密码明文直接 false | `verify_password` | 输入边界和 false 语义测试 |
| FND-041 | PHC 必须能规范的往返一致性 | `hash.to_string() == encoded` | 非规范的编码拒绝 |
| FND-042 | 参数不同的有效 Argon2id hash也拒绝 | `password_hash_uses_current_policy` | weak/alternate 规则负例 |
| FND-043 | 可在无密码明文时检查持久 hash 当前性 | `require_current_password_hash` | 产品启动遍历所有管理员并 fail fast |
| FND-044 | xcss 不实现“成功登录后顺便升级 hash” | API 明确无此入口 | API 面/依赖扫描；转换归独立流程 |

## 令牌、Cookie、同源与 CSRF：`xcss::admin_auth`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-050 | 通用令牌使用 32-byte OS 随机源；当前 CSRF 从会话令牌稳定派生 | `random_token`、`getrandom::fill`、`derive_csrf_token` | 随机源失败无回退；派生值与会话绑定测试 |
| FND-051 | 令牌使用 URL 安全的 Base64无填充 | `URL_SAFE_NO_PAD` | 编码字符集与无 `=` 测试 |
| FND-052 | 当前令牌字符串恰好 43 字节 | `SESSION_TOKEN_ENCODED_BYTES`、`is_token_shape` | 42/43/44 边界 |
| FND-053 | 令牌必须解码后再规范的 re-encode相同 | `is_token_shape` | 非规范最后字符负例 |
| FND-054 | 令牌摘要固定 SHA-256 32 字节 | `token_hash`、`TOKEN_HASH_BYTES` | golden digest 与长度测试 |
| FND-055 | 提供小写 hex 令牌 hash | `token_hash_hex` | 64 位小写 hex |
| FND-056 | 令牌比较先验证 shape 与 expected hash长度 | `token_matches_hash` | non规范令牌/31/33-byte digest负例 |
| FND-057 | 令牌 digest 比较使用 constant time | `subtle::ConstantTimeEq` | code review + match正反测试 |
| FND-058 | Cookie 名称仅允许字母数字/下划线/连字符 | `parse_cookie_value` | 非法 name负例 |
| FND-059 | Cookie 只返回唯一非空同名值 | `parse_cookie_value` | 空值/重复值/空格分隔负例 |
| FND-060 | 框架必须自行拒绝重复 Cookie field line | `parse_cookie_value` doc contract | 产品集成重复 header测试 |
| FND-061 | 安全 header 辅助函数要求恰好一个 field-line value | `require_single_security_header_value` | missing/2行/3行具有类型约束的 error |
| FND-062 | 安全 header 只接受 nonempty visible ASCII | 同上 | 空、space、控制字符、non-UTF8负例 |
| FND-063 | 安全 header 拒绝逗号连接值 | 同上 | comma负例 |
| FND-064 | 同源校验显式区分 Production HTTPS 与回环地址 HTTP dev | `AdministratorOriginMode` | 两模式 scheme/host矩阵 |
| FND-065 | Production Origin 协议方案必须精确 `https` | `mode.scheme()` | http production负例 |
| FND-066 | Development Origin 协议方案必须精确 `http` | `LoopbackDevelopmentHttp` | https dev负例 |
| FND-067 | HTTP dev双方 host 必须是 localhost/127.0.0.0/8/::1 | `NormalizedHost::is_loopback` | 非回环地址 DNS/IP负例 |
| FND-068 | Origin、Host、Sec-Fetch-Site 都必须出现一次 | `require_administrator_same_origin` | 三类 missing/duplicate负例 |
| FND-069 | `Sec-Fetch-Site` 必须精确 `same-origin` | 同上 | 值枚举负例 |
| FND-070 | Origin 必须有且只有 `scheme://authority` | `split_once` + `parse_authority` | 各 URL 部件攻击负例 |
| FND-071 | Host 支持 DNS、规范的 IPv4、bracketed IPv6 | authority 解析器 | DNS/v4/v6正例 |
| FND-072 | DNS host比较时小写，拒绝首尾点/非法 label | `parse_unbracketed_host` | case正例、尾点/非法字符负例 |
| FND-073 | IPv4/IPv6 必须采用规范的文本 | authority 解析器、`to_string` | 非规范的 IP负例 |
| FND-074 | 端口 1..65535、无前导零，缺失时用协议方案默认 | `parse_port`、`default_port` | 端口边界矩阵 |
| FND-075 | Origin authority 与有效 Host authority精确相等 | `OriginHostMismatch` | 协议方案默认端口与显式端口测试 |
| FND-076 | 调用方必须把 HTTP/2 URI authority加入有效 Host值集合 | API doc、消费者适配器 | authority-only正例、二者冲突负例 |
| FND-077 | 不解析 `X-Forwarded-*` 或信任代理回退 | API 明确排除 | API 面扫描；产品先形成权威 Host |
| FND-078 | CSRF header必须唯一、visible且为规范令牌 | `require_single_csrf_token` | missing/duplicate/comma/shape负例 |
| FND-079 | CSRF 与会话 digest常量时间比较 | `require_csrf_token_matches_hash` | match/mismatch/错误长度测试 |
| FND-080 | `admin_auth` 提供安全原语；上层公共模块固定 Cookie、TTL 和会话存储机制 | `admin_auth`、`admin_core`、`admin_sqlite`、HTTP 适配器 | 公共 Cookie/TTL/撤销测试与产品挂载、部署验收 |

## 管理员与错误通信格式 contract：`xcss::contracts` / `@xcss/web/contracts` / `xcss::error`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-090 | 三个管理员认证 path 是跨 Rust/TS常量 | `ADMIN_*_PATH`、`ADMIN_AUTH_PATHS` | 恰好 `/api/v1/auth/login\|session\|logout` |
| FND-091 | `AdministratorLoginRequest` 精确的两字段，Rust 读写均先验证 | Rust custom serialize/deserialize、TS 运行时校验、Schema | missing/unknown/type 测试夹具；非法 public value 序列化失败 |
| FND-092 | 登录 username 候选 1..64 可打印 ASCII 字节 | Rust custom deserialize、TS 运行时校验、Schema | empty/64/65/control/DEL/Unicode 负例 |
| FND-093 | 登录 password候选 1..1024 码点且无控制字符 | 同上 | empty/1025/control负例；服务端再按字节校验 |
| FND-094 | 管理角色枚举只有 `admin` | `AdministratorRole::Admin`、常量/Schema const | viewer/operator/其他值拒绝 |
| FND-095 | 会话恰好五字段 | `AdministratorSession`、运行时校验、Schema | 精确的 keys 测试夹具 |
| FND-096 | `authenticated` 必须为 literal true | custom deserialize/serialize、TS 运行时校验 | false/null/missing负例 |
| FND-097 | `user_id` 是有界ASCII identifier | contracts 验证器 | 1..128、字符集边界 |
| FND-098 | 会话 username 必须已是规范管理员用户名 | admin-auth复用、TS regex、Schema | Rust/TS/Schema 同一测试夹具；uppercase/`@`拒绝 |
| FND-099 | 会话 `csrf_token` 必须当前43字符令牌 | admin-auth复用、TS 运行时校验、Schema | 规范的末尾bits负例 |
| FND-100 | Rust 会话 constructor自动固定 `role=admin` | `AdministratorSession::new` | serialize验证和constructor测试 |
| FND-101 | `ErrorCode` 1..128 字节、小写字母开头 | `xcss::error::ErrorCode`、TS 运行时校验 | 首字符/长度/Unicode负例 |
| FND-102 | ErrorCode只含小写字母、数字、`.`、`_`、`-` | ErrorCode 验证器 | 字符集测试夹具 |
| FND-103 | `RequestId` 1..128 字节有界ASCII identifier | `RequestId`、TS `isRequestId` | 空/129/control/Unicode负例 |
| FND-104 | 错误响应结构拒绝未知 fields | Rust `deny_unknown_fields`、TS allowed keys、Schema | unknown/missing 测试夹具 |
| FND-105 | 错误响应结构固定 code/message/retryable | `ErrorEnvelope` | required字段正反测试 |
| FND-106 | `request_id` 缺失与存在有效值区分，显式null拒绝 | custom deserialize、guard/Schema | missing/null/invalid 测试夹具 |
| FND-107 | `details` 只能是对象，空对象序列化省略 | `Map<String,Value>`、guard/Schema | array/scalar负例；空值 serialization |
| FND-108 | message只用于展示，machine分支使用code/status | 类型doc与http-client | 消费者不按message分支审查 |
| FND-109 | 9个常用HTTP status与code映射 | `HttpStatus` | 400/401/403/404/409/422/429/500/503测试 |
| FND-110 | 默认仅429/503可重试 | `default_retryable` | status映射测试；产品可显式覆盖 |
| FND-111 | Rust error类型可构造、parse、display与serde | impl集合 | 往返一致性与invalid serde测试 |
| FND-112 | Rust/TS/Error Schema共用相同测试夹具 | contracts 测试夹具 + Rust include | valid/invalid在两端全部执行 |

## 状态、发行与备份合同

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-120 | 状态合同通信格式 version固定1 | `STATE_CONTRACT_VERSION`、Schema/guard | 其他version拒绝 |
| FND-121 | State 精确的 fields且未知拒绝 | Rust serde、TS 运行时校验、Schema | missing/unknown 测试夹具 |
| FND-122 | application/version为有界identifier | contract validators | 字符与长度边界 |
| FND-123 | source_revision恰好40位小写hex | custom deserialize、TS 运行时校验、Schema | 39/41/uppercase/nonhex负例 |
| FND-124 | State schema字段必须出现，可为null | custom required option、运行时校验精确的 keys | missing/null/object三态测试夹具 |
| FND-125 | State schema 修订号为非负安全整数 | custom 安全整数、运行时校验 | MAX_SAFE/+1/fraction/negative负例 |
| FND-126 | State schema SHA为64位小写hex | 验证器 | hash边界测试夹具 |
| FND-127 | maintenance lock为唯一identifier数组 | unique 验证器 | duplicate/invalid lock负例 |
| FND-128 | State resource kind只有5种当前值 | `StateResourceKind`、TS union、Schema | 未知 kind拒绝 |
| FND-129 | State resource恰好name/kind/required | struct/guard/Schema | exact-field 测试夹具 |
| FND-130 | External requirement描述kind/kid/algorithm/envelope_version | contract struct/guard | identifier与positive 安全整数测试 |
| FND-131 | Companion contract绑定name/version/platform/SHA | contract struct/guard | 精确的 fields/hash 测试夹具 |
| FND-132 | 发行身份恰好五字段 | `ReleaseIdentity`、Schema/guard | 精确的 key 测试夹具 |
| FND-133 | 发行 product/version/target为identifier | validators | invalid identifier负例 |
| FND-134 | 发行绑定完整源码修订号 | `source_revision` | 40位SHA与tag/HEAD复核 |
| FND-135 | 发行用state_contract_sha256绑定状态合同 | identity字段、asset builder | build顺序和hash drift负例 |
| FND-137 | 备份清单通信格式 version 固定为 `1` | `BACKUP_MANIFEST_VERSION`、Rust/TS/Schema | 其他 version 拒绝；真实业务备份仍由产品实现 |
| FND-138 | Backup 精确的 fields且未知拒绝 | Rust/TS/Schema | shared 测试夹具 |
| FND-139 | Backup 结构身份复用完整四字段SchemaIdentity | type 别名与Schema | null/object及exact-current由产品加强 |
| FND-140 | Backup created_at为非负safe epoch seconds | 安全整数验证器 | max/+1/negative/fraction负例 |
| FND-141 | Backup resources至少一项 | custom deserialize/validate、运行时校验 | 空值 array负例 |
| FND-142 | Backup resource path非空但不声称规范的 | `validate_non_empty_path` | 空值拒绝；产品继续验证canonical/path root |
| FND-143 | Backup 字节非负safe、files正安全整数 | validators | 0 files、un安全整数负例 |
| FND-144 | Backup每个资源有SHA-256 | `BackupResource` | hash格式 + 产品实际hash复核 |
| FND-145 | Backup external requirement额外绑定Secret材料SHA | `BackupExternalRequirement` | sha/algorithm/envelope 精确的测试 |
| FND-146 | Rust State/Release/Backup直接include TS 软件包测试夹具 | `include_str!`测试 | 同一valid/invalid集合 |
| FND-147 | JSON Schema与测试夹具作为公开软件包导出 | contracts manifest/copy script | 发行归档导出入口解析 |

## Schema 身份：`xcss::schema_identity`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-150 | Schema算法与数据库driver解耦 | 模块算法不调用SQLx/rusqlite，单体包统一持有服务端依赖 | 依赖树、双driver消费者 |
| FND-151 | 规范的 `product_metadata` DDL恰好五列 | `PRODUCT_METADATA_DDL` | DDL 测试夹具与产品migration对比 |
| FND-152 | singleton INTEGER PK NOT NULL CHECK=1 | DDL/column 验证器 | DDL/PRAGMA/row负例 |
| FND-153 | application/application_version TEXT NOT NULL | DDL/column 验证器 | column和runtime typeof检查 |
| FND-154 | schema_revision INTEGER NOT NULL | DDL/column 验证器 | column/storage/negative测试 |
| FND-155 | schema_sha256 TEXT NOT NULL | DDL/column 验证器 | column/storage/hash测试 |
| FND-156 | 五列顺序/cid/type/notnull/pk/default精确验证 | `validate_product_metadata_columns` | 每个列属性漂移负例 |
| FND-157 | DDL比较只忽略ASCII空白与字母case | `validate_product_metadata_ddl` | check/default/列变化负例 |
| FND-158 | 元数据必须恰好一row | `schema_identity_from_metadata_rows` | 0/2 row 具有类型约束的 error |
| FND-159 | singleton值必须精确1 | `ProductMetadataRow::to_schema_identity` | 0/2负例 |
| FND-160 | schema 修订号不能为负 | i64→u64 checked conversion | -1负例 |
| FND-161 | identity四分量都参与精确的当前比较 | `SchemaIdentity::require_exact` | 每字段mismatch 具有类型约束的 error |
| FND-162 | identifier 1..128 ASCII安全字符 | identity 验证器 | 字符/长度测试 |
| FND-163 | schema SHA必须64位小写 hex | `validate_schema_sha256` | upper/length/nonhex负例 |
| FND-164 | 结构指纹算法版本常量为1 | `SCHEMA_FINGERPRINT_ALGORITHM_VERSION` | golden vector版本 |
| FND-165 | 查询排除 `sqlite_*` 内部对象 | `SQLITE_SCHEMA_ROWS_QUERY` +函数二次拒绝 | excluded row负例 |
| FND-166 | 查询排除 `product_metadata` 自身 | query +函数二次拒绝 | included 元数据负例 |
| FND-167 | 行按 type/name/table的BINARY顺序 | query ORDER BY +函数检查 | 乱序负例 |
| FND-168 | 重复 Schema object key拒绝 | `DuplicateSchemaObject` | 重复负例 |
| FND-169 | 每row四字段分别加入u64 大端序字节长度 | `schema_fingerprint` framing | golden framing vectors |
| FND-170 | SQL按原始UTF-8 字节进入hash | `digest.update(bytes)` | 空白/Unicode SQL差异vector |
| FND-171 | 声明结构指纹与实际结构指纹双重比较 | `verify_fingerprint`/`verify_current_schema` | declared/actual mismatch负例 |
| FND-172 | 发布可复核golden vectors JSON | crate 测试夹具常量 | 测试夹具 parse与expected hash |
| FND-173 | 具有类型约束的 error指出row/column/identity/order/hash字段 | `Error`/`IdentityField` | error variant单元测试 |

## SQLx SQLite 基线：`xcss::sqlite`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-180 | `open_existing` 明确拒绝缺失数据库 | path `try_exists` + create=false | 缺失具有类型约束的 error；产品先做不跟随符号链接的 |
| FND-181 | `create_if_missing` 是单独显式入口 | create=true API | 缺失创建、existing打开测试 |
| FND-182 | pool max connections必须大于0 | `PoolOptions::validate` | 0负例 |
| FND-183 | min connections不能超过max | 同上 | 边界负例 |
| FND-184 | acquire 超时必须非零，默认10秒 | options/常量 | zero/default/custom测试 |
| FND-185 | 每连接强制foreign_keys=ON | `SqliteConnectOptions` | 多连接PRAGMA与FK violation |
| FND-186 | 每连接强制WAL | journal mode | PRAGMA实际值 |
| FND-187 | 每连接强制synchronous=FULL | synchronous mode | PRAGMA实际值 |
| FND-188 | busy 超时固定5秒 | `BUSY_TIMEOUT` | PRAGMA/锁竞争测试 |
| FND-189 | 完整性摘要 check必须唯一返回`ok` | `integrity_check` | 多诊断/损坏测试夹具 |
| FND-190 | FK check收集table/row/parent/index | `ForeignKeyViolation` | 具体violation断言 |
| FND-191 | TRUNCATE checkpoint busy是失败 | `CheckpointBusy` | busy 读取器测试 |
| FND-192 | checkpoint frame不完整也是失败 | `CheckpointIncomplete` | tuple 验证器负例 |
| FND-193 | schema rows可从pool/connection/transaction executor读取 | generic `Executor<Sqlite>` | 三类调用/编译测试 |
| FND-194 | SQLx 适配器复用纯结构指纹算法 | `fingerprint_rows` | shared golden/current schema测试 |
| FND-195 | 读取元数据前同时校验DDL、列与存储类别 | `validate_metadata_table`/`typeof` query | DDL/PRAGMA/typeof负例 |
| FND-196 | `read_schema_identity` 先验证实际hash才返回身份 | function顺序 | drifted DDL测试 |
| FND-197 | pool convenience仍执行相同当前验证 | `read_pool_*`/`require_pool_*` | 封装集成测试 |
| FND-198 | `sqlite` 模块不定义业务 DDL、自动迁移或业务备份 | `xcss::sqlite` 模块 API | 文件安全和实例锁由其他公共模块提供；产品仍验证业务生命周期 |
| FND-199 | `xcss::platform_db` 提供当前平台元数据单例的事务内初始化、严格读取与运行形态验证；产品仍拥有业务初始数据 | `initialize_current_platform_metadata`、`require_current_platform_metadata` | 新库初始化后立即验证；空行、重复行、错误 Profile/代际拒绝 |

## 服务端架构门禁：`xcss::server_target`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-200 | 服务端唯一target为`x86_64-unknown-linux-gnu` | `SERVER_TARGET_TRIPLE` | 常量、发行身份、文档一致 |
| FND-201 | 目标架构必须为 x86_64 | compile-time cfg | aarch64 编译失败 |
| FND-202 | 目标操作系统必须为 Linux | compile-time cfg | cross-OS 编译失败 |
| FND-203 | target env必须GNU/glibc | compile-time cfg | musl 编译失败 |
| FND-204 | pointer width必须64 | compile-time cfg | cfg gate |
| FND-205 | 非目标在编译期直接`compile_error!` | crate root | 负值 target job/手动编译 |
| FND-206 | 提供human architecture常量`amd64` | `SERVER_ARCHITECTURE` | 常量测试 |
| FND-207 | runtime/release 元数据可精确检查target | `require_server_target` | accepted 规范的、所有近似值拒绝 |
| FND-208 | 客户端/Client不依赖该crate | crate文档与消费者Cargo边界 | 依赖 tree + 各客户端平台构建 |
| FND-209 | xcss整体只允许Linux AMD64 GNU编译及发布 | 发行 target `x86_64-unknown-linux-gnu` | 发行身份检查 |

## JSON HTTP 客户端：`@xcss/web/http-client`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-220 | `requestJson<T>` 是唯一请求入口 | 软件包导出 | export/tarball测试 |
| FND-221 | 无浏览器location时必须显式绝对baseUrl | `resolveSameOriginUrl` | no-location负例 |
| FND-222 | URL只允许HTTP/HTTPS | `parseHttpUrl` | 协议方案负例且fetch未调用 |
| FND-223 | URL拒绝控制字符和空值 | `parseHttpUrl` | control/empty负例 |
| FND-224 | URL拒绝用户信息 | username/password检查 | 用户信息负例 |
| FND-225 | target 来源与base 来源精确相等 | 来源比较 | scheme/host/port变化负例 |
| FND-226 | 重定向固定`error`且不允许调用方放宽 | request validation/fetch init | override负例、capture init |
| FND-227 | 凭据默认`same-origin` | fetch init | fetch capture |
| FND-228 | 默认Accept application/json，保留其他header | Headers logic | header merge测试 |
| FND-229 | method必须string并统一大写判断安全性 | method 验证 | method type/case测试 |
| FND-230 | 只对非安全 HTTP 方法注入CSRF | `isUnsafeMethod` | GET/HEAD/OPTIONS/TRACE vs POST/PUT/PATCH/DELETE |
| FND-231 | CSRF必须非空且无CR/LF | 令牌验证 | empty/newline负例 |
| FND-232 | 超时默认10秒、范围1..120000ms 安全整数 | constants/`validateBudget` | 上下界/fraction/NaN负例 |
| FND-233 | 调用方 AbortSignal与超时 first-wins合并 | local AbortController/source | caller-first/timeout-first交错测试 |
| FND-234 | finally清timer和调用方 listener | `finally` | fake timer/listener测试 |
| FND-235 | network/timeout/caller abort有不同具有类型约束的 code | catch/localError | 三类失败测试 |
| FND-236 | 成功响应默认2MiB、最大64MiB | constants/budget 验证 | exact/+1和配置上限测试 |
| FND-237 | 错误正文独立固定64KiB上限 | `MAX_ERROR_RESPONSE_BYTES` | oversize error测试 |
| FND-238 | 先拒绝过大声明Content-Length并取消body | `readBoundedText` | declared size/cancel测试 |
| FND-239 | 再流式累计实际Uint8Array 字节 | reader/chunks | chunked exact/+1测试 |
| FND-240 | 超限时读取器 cancel失败不覆盖权威size error | nested try/catch | cancel throw测试 |
| FND-241 | UTF-8使用fatal 解码 | `TextDecoder(...,{fatal:true})` | 畸形 UTF-8测试 |
| FND-242 | success/error只接受JSON或`+json`Content-Type | `isJsonContentType` | MIME参数、html/plain负例 |
| FND-243 | HEAD/204/205/Content-Length 0返回undefined | 空值 response分支 | 各空响应测试 |
| FND-244 | 非2xx严格解析共享错误响应结构 | `responseError` + 运行时校验 | valid/invalid envelope测试 |
| FND-245 | 非法error body不回显raw HTML/Secret | 通用安全错误 | sentinel secret不可见断言 |
| FND-246 | body request_id优先、header 回退且均严格过滤 | safeRequestId/responseError | body/header优先级与非法值测试 |
| FND-247 | Retry-After支持safe整数秒并封顶24h | 解析器 | 0/large/unsafe测试 |
| FND-248 | Retry-After日期只接受规范HTTP-date 往返一致性 | regex/Date/UTCString | canonical/noncanonical日期测试 |
| FND-249 | 401 callback被等待完成 | response path | async callback顺序测试 |
| FND-250 | 401 callback异常不覆盖权威API error | callback catch | throwing callback测试 |
| FND-251 | 从不自动重试 | API无重试 loop | fetch调用次数负例 |
| FND-252 | 泛型T不声称运行时验证 | JSON parse cast + docs | 产品运行时校验集成测试 |
| FND-253 | 不用于文件上传/下载或streaming媒体 | 软件包边界 | API面与产品专用transport |

## 管理员 Web：`@xcss/web/admin-web`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-260 | 统一精确 Node/React/Vite/TS 工具链常量 | `web/web-toolchain/src/index.ts` 的 `WEB_TOOLCHAIN` | 常量、根清单及实际消费者构建验证 |
| FND-261 | 清单 assertion检查Node engine精确字符串 | `assertXcssWebToolchain` | 任何范围变化拒绝 |
| FND-262 | Node 版本在去除首尾空白后必须为精确值 | `assertXcssWebToolchain` 的 `nodeVersion.trim()` | 精确值与额外版本文本负例；上游 `.node-version` 由仓库规则另验 |
| FND-263 | dependencies/devDependencies 中出现工具链包时要求精确版本 | `assertXcssWebToolchain` 的两类清单遍历 | 两类 section 漂移负例；上游 peer、平台元数据和 lock 由仓库及打包规则另验 |
| FND-264 | 不强制消费者使用pnpm | assertion只检查工具链，不查packageManager | xsos/xscs/xszs/xcos npm lock验证 |
| FND-265 | 浏览器默认baseUrl为当前页面来源根 | `resolveAdministratorBaseUrl` | browser location测试 |
| FND-266 | 浏览器显式baseUrl仍必须同来源 | base/当前页面来源比较 | 跨源负例 |
| FND-267 | baseUrl只允许HTTP(S)且无用户信息 | URL 验证 | protocol/userinfo负例 |
| FND-268 | Node/nonbrowser必须显式baseUrl | runtime location 运行时校验 | no-location负例 |
| FND-269 | 业务path必须以`/api/v1/`开头 | `resolveApiPath` | prefix/control/hash负例 |
| FND-270 | path解析后仍检查来源与pathname | URL revalidation | origin/prefix攻击负例 |
| FND-271 | 调用方不能自设`X-CSRF-Token` | send header ownership检查 | existing header负例 |
| FND-272 | body存在且无Content-Type时自动JSON | send header逻辑 | capture测试；保留调用方明确值 |
| FND-273 | 凭据只能未设置或同源 | send init检查 | override负例 |
| FND-274 | 成功响应必须通过调用方运行时校验 | `send<T>` | false 运行时校验产生`invalid_response_shape` |
| FND-275 | Session/CSRF仅保存在closure内存 | `session`/`transportSession`局部变量 | 源码扫描无storage；重载需恢复会话 |
| FND-276 | publish对所有订阅者同步通知并可unsubscribe | listener Set | subscribe/unsubscribe测试 |
| FND-277 | 登录 request先经过共享候选运行时校验 | `isAdministratorLoginRequest` | client拒绝且fetch未调用 |
| FND-278 | login/logout认证修改操作全局串行 | `authenticationMutationTail` | overlap顺序测试 |
| FND-279 | 每次login/logout/当前401推进generation | `authenticationGeneration` | delayed response竞态测试 |
| FND-280 | 登录开始立即清旧UI 会话 | `publish(null)` | transition测试 |
| FND-281 | `transportSession`只服务排队退出的CSRF | private snapshot | overlapping login/logout测试 |
| FND-282 | superseded 认证 operation返回具有类型约束的 client error | `auth_operation_superseded` | generation测试 |
| FND-283 | 并发恢复会话复用同一Promise | `restorePromise` | fetch调用一次测试 |
| FND-284 | 恢复会话等待之前排队的认证修改操作 | `precedingMutations` | overlap测试 |
| FND-285 | 恢复会话仅在当前generation发布会话 | `requireCurrentOperation` | delayed 恢复会话测试 |
| FND-286 | invalid 会话 response清当前状态 | 恢复会话 catch code集合 | shape/content/json/size错误测试 |
| FND-287 | 401只在dispatch 会话仍为当前时invalidate | 认证 context比较 | stale 401竞态测试 |
| FND-288 | 退出本地授权同步结束，服务端请求仍排队完成 | 退出流程 | immediate state + network顺序测试 |
| FND-289 | 退出失败保留内存撤销目标供主动重试，确认成功或明确失效后才清理 | `logout`、私有 `transportSession` | 网络失败、重试成功、明确 401 失效与竞态测试；不自动无限重试 |
| FND-290 | React 钩子提供五态可区分联合类型 | `AdministratorSessionState` | loading/anonymous/anonymous_logout_unconfirmed/authenticated/error 类型与行为测试 |
| FND-291 | 钩子 mount自动恢复会话 | `useEffect` | lifecycle测试 |
| FND-292 | 钩子用active client ref阻止换client后的旧更新 | `activeClient` | client swap测试 |
| FND-293 | 钩子用state generation阻止旧Promise覆盖 | `stateGeneration` | 交错执行测试 |
| FND-294 | 钩子把401/superseded归anonymous，其余恢复会话失败归error | catch分类 | error分类测试 |
| FND-295 | Vite 辅助函数固定 React plugin、dist 与 emptyOutDir，并限制资源大小 | `web/web-toolchain/src/vite.ts` 的 `createXcssReactViteConfig` | 配置与资源预算边界测试；实际产品 build |
| FND-296 | Vite 选项允许 base 和 maxAssetBytes | 函数参数 `{base?: string; maxAssetBytes?: number}` | 预算默认 512 KiB，允许 1 字节至 64 MiB；不扩展为产品业务构建配置 |
| FND-297 | Xczs 采用 React/Vite 管理外壳并组合原生文件业务模块 | Xczs `web-react-admin` 清单与管理 Web | 同一构建、认证与嵌入合同；文件/上传控制器保持产品业务验收 |

## 设计原语：`@xcss/web/design-tokens`

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-300 | TS导出小型颜色/间距/圆角原语 | `src/index.ts` | declaration/export/test |
| FND-301 | light语义令牌与原语对齐 | `semanticTokens.light`、`tokens.css` | source/CSS effective value测试 |
| FND-302 | dark只覆盖真正变化的语义值 | `semanticTokens.dark`、`tokens.dark.css` | light+override effective测试 |
| FND-303 | CSS变量统一`--xcss-*` 命名空间 | 令牌 CSS | selector/property扫描 |
| FND-304 | reset只在`[data-xcss-scope]`内生效 | `reset.css` | 禁止无作用域全局selector测试 |
| FND-305 | box-sizing对作用域及后代统一 | 限定作用域的 reset | CSS source/dist测试 |
| FND-306 | 表单继承font且disabled cursor明确 | reset | CSS规则测试 |
| FND-307 | heading/text wrap与hidden基线 | reset | CSS规则测试 |
| FND-308 | `:focus-visible`提供清晰键盘焦点 | accessibility CSS | CSS + 产品键盘测试 |
| FND-309 | reduced-motion显著压缩动画/transition | media query | media query/source-dist测试 |
| FND-310 | forced-colors使用系统Highlight | media query | CSS + 人工高对比度验证 |
| FND-311 | visually-hidden保留辅助技术文本 | utility class | 精确的 CSS测试 |
| FND-312 | 清理后构建删除陈旧CSS/JS声明 | clean/copy scripts | stale artifact负例 |
| FND-313 | design-tokens 模块只提供原语，不定义产品品牌、主题状态或 CDN | `@xcss/web/design-tokens` 子路径边界 | 同包 admin-ui/admin-shell/web-fonts 提供公共组件、外壳与字体；产品品牌仍独立 |

## 软件包、发布树与供应链

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-320 | 单个npm 软件包统一metadata/license/engine/repository | 清单 + 规则 | 清单 check |
| FND-321 | 软件包只发布`dist` | `files:["dist"]` | tar inventory |
| FND-322 | 每个公开入口显式写入导出入口 | 软件包清单 | import.meta.resolve全部导出入口 |
| FND-323 | contracts复制5份Schema与5份测试夹具 | copy-contract-data script | dist/tar 导出入口检查 |
| FND-324 | JS/declaration、CSS/Schema/fixture/font 等资源按模块显式构建与复制 | 根构建脚本、模块复制脚本；tsconfig 由 web-toolchain 提供 | dist 校验与真实 tar 成员、导出目标检查 |
| FND-325 | design-token复制4份CSS入口 | copy-css script | source=dist/tar测试 |
| FND-326 | build前clean且拒绝linked dist | 软件包 artifact 规则 | linked/stale dist负例 |
| FND-327 | 清单与导出入口目标必须普通单链接文件 | artifact 规则 | link负例 |
| FND-328 | 内部模块使用同包公开子路径 | manifests/policy | packed 清单检查 |
| FND-329 | 内部软件包依赖与工作区协议被删除 | 清单规则 + packed check | tgz 清单无工作区 |
| FND-330 | tar path必须规范的且留在软件包根 | tar inspector | absolute/`..`/backslash负例 |
| FND-331 | tar拒绝重复 member | tar inspector | 重复负例 |
| FND-332 | tar拒绝link和special file | tar inspector | symlink/hardlink/device负例 |
| FND-333 | tar拒绝意外源码/未发布文件 | expected inventory | member allowlist |
| FND-334 | 临时空消费者安装唯一真实tgz | 软件包冒烟验证 | npm offline/ignore-scripts install |
| FND-335 | 离线消费者解析每个静态/代码导出入口 | 冒烟验证 import/resolve | 全导出入口遍历 |
| FND-336 | 发行 tool BuildIdentity复用五字段合同 | `xcss_release::BuildIdentity` | 往返一致性/fixture |
| FND-337 | 发行清单记录精确的 path/mode/size/hash | release.py | build/parse/verify测试 |
| FND-338 | 发行 path拒绝absolute/parent/backslash/NUL/超长 | path 验证器 | 攻击测试夹具 |
| FND-339 | 发行目录只允许普通单链接文件 | lstat/不跟随符号链接的规则 | link/device负例 |
| FND-340 | 不跟随符号链接的 fd hash并复核inode/size/time/path | safe hashing | race replacement/growth测试 |
| FND-341 | file/tree/count/manifest/size都有硬上限 | 发行规则 constants | limit与fail-fast调用次数测试 |
| FND-342 | tool tar固定mtime=0/uid/gid/name/mode/order | asset builder | 双构建SHA相同 |
| FND-343 | 发行 output必须安全、空、非root目录 | `prepare_output` | broad/nonempty/link目录负例 |
| FND-344 | 发行要求工作树无tracked/untracked变化 | `verify_source` | porcelain必须空 |
| FND-345 | 标签必须精确`v1.0.2`且唯一指向HEAD | `verify_source` | wrong/missing/multiple 标签负例 |
| FND-346 | 状态合同先生成再hash绑定发行身份 | asset builder顺序 | hash与Schema验证 |
| FND-347 | build inventory记录工具链与两个lock hash | `inventory` | JSON内容与hash测试 |
| FND-348 | inventory枚举1个Rust和1个npm组件 | cargo 元数据 + 软件包 discover | component集合检查 |
| FND-349 | `SHA256SUMS`覆盖全部非自身artifact | `checksums` | asset集合与sha256sum复核 |
| FND-350 | release-tree 清单置于artifacts外且不自描述 | builder布局 | 精确的 tree测试 |
| FND-351 | action只允许完整SHA allowlist | 工作流校验规则 | valid/invalid 测试夹具 |
| FND-352 | runner固定ubuntu-24.04且job 超时有界 | 工作流校验规则 | floating/missing/oversize负例 |
| FND-353 | 顶层权限空，普通job只读contents | 工作流校验规则 | write permission负例 |
| FND-354 | 源码检出不持久化凭据 | 工作流校验规则 | missing/true负例 |
| FND-355 | setup-node固定26.7.0且不check-latest | 工作流校验规则 | floating/check-latest负例 |
| FND-356 | workflow拒绝YAML anchor/alias/merge及伪造action位置 | textual policy/tests | 专用测试夹具 |
| FND-357 | 唯一写权限只在tag-only 发行 job | 工作流校验规则 | 文件/job/trigger三重负例 |
| FND-358 | Cargo/pnpm lock提交并使用locked/frozen | lockfiles、CI命令 | clean 源码检出验证 |

## 测试、消费者与文档保障

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-360 | Rust执行fmt/check/clippy/test/doc完整门禁 | CI/README/operations | locked/all-target/all-feature/-Dwarnings |
| FND-361 | Web执行typecheck、软件包 unit与tar 冒烟验证分层 | pnpm scripts/package tool | 干净工作区 + offline 冒烟验证 |
| FND-362 | Python policy/release/package有正反单元测试 | `tools/tests` | unittest discovery全部通过 |
| FND-363 | 管理员认证有策略/攻击/authority/cookie/CSRF测试 | `xcss::admin_auth` tests | 每种具有类型约束的 error和边界 |
| FND-364 | admin-web有 login/logout/restore/401 的受控 Promise 竞态测试 | 软件包 tests | controlled Promise/fetch交错；React client-switch 仍由消费者组件门禁验证 |
| FND-365 | contracts用跨语言共享测试夹具 | contracts tests/crate include | valid/invalid全量执行 |
| FND-366 | design 令牌测试source/dist/value/scope | 软件包 test/scripts | CSS静态与effective值测试 |
| FND-371 | 中文文档限定五类并与源码同步 | README/docs结构 | 链接/API/版本/命令抽查 |
| FND-372 | 功能台账逐项记录删除后果与验证边界 | 本文件 | PR评审要求唯一ID同步 |

### 正式内嵌 Web 与共同构建

| ID | 当前功能/特性 | 实现/锚点 | 最低验证/边界 |
| --- | --- | --- | --- |
| FND-400 | 正式 Web 字节内嵌且与服务端同一身份 | `xcss::web_assets::build`、ADR-0009 | 编译资源快照、清单摘要、实际 binary 验收 |
| FND-401 | 清单自动生成并绑定路径/MIME/大小/SHA-256 | `EmbeddedAsset`、`verify_embedded` | 排序、规范的 JSON、篡改与资源遗漏负例 |
| FND-402 | GET/HEAD/304、准确 MIME 与缓存统一 | `xcss::web_assets::response` | HEAD 空 body、ETag 列表/weak/*、HTML no-store |
| FND-403 | 明确开发目录与每次请求安全读取 | `DirectoryAssets` | 热更新、Unix descriptor/不跟随符号链接的、链接/路径穿越负例 |
| FND-404 | 前端→Rust→实际资源的共同构建顺序 | `@xcss/web/web-toolchain/server`、`xcss-build-server` | 干净源码、锁定依赖、规范 target、执行 binary `web-assets` |
| FND-405 | capability 与构建声明接入门禁 | `xcss_conformance.policy` | 缺失 capability/declaration、目录逃逸、缺共同 runtime/build crate 负例 |

## 明确排除项（同样属于功能边界）

| ID | 不进入 xcss 的能力 | 当前归属/实现锚点 | 验证边界 |
| --- | --- | --- | --- |
| FND-380 | 产品业务账户、设备身份和专有账号生命周期 | 各产品业务 DB/CLI/API | 单管理员初始化、自助更新、当前身份与密码策略由 admin_core/admin_sqlite 固定，产品验证业务身份 |
| FND-381 | 业务会话或产品专有授权状态 | 各产品数据面 | 管理员 TTL、并发上限与撤销由公共模块固定；产品测试数据面授权 |
| FND-382 | 产品代理、域名、监听和 TLS 部署策略 | 各产品部署层与 HTTP 挂载 | 管理 Cookie 名、Path/Secure/HttpOnly/SameSite 由公共适配器固定；产品验证真实 Set-Cookie/TLS |
| FND-383 | 产品业务请求的限流、容量和审计规则 | 各产品业务接口 | 公共管理员登录限流、未知用户成本与安全审计不删除；产品攻击和容量测试保留 |
| FND-384 | 设备、客户端、API key、媒体令牌等数据面身份 | 各产品协议 | admin 通信格式与数据面合同分离测试 |
| FND-385 | 产品业务路由、DTO、middleware 和额外响应约束 | 各服务端业务 HTTP 层 | admin_axum/admin_hyper/server_cli 提供公共管理面、body/rejection/request-ID；产品真实路由测试 |
| FND-386 | 产品配置字段、环境映射与外部 Secret 来源选择 | 各产品 config/env 与 Secret backend | config/secret 模块提供安全原语；产品启动字段、权限与外部前提负例 |
| FND-387 | 产品文件根、媒体对象和专有资源访问策略 | 各产品资源层 | fs_safety/state_file 提供 no-follow、openat2、owner/mode 和锁机制；产品仍验证授权与资源边界 |
| FND-388 | 产品业务 SQLite DDL、业务事务与业务 writer | 各产品业务数据库 | platform_db/admin_sqlite/operations 提供公共表和事务；产品拥有业务 DDL 与并发语义 |
| FND-389 | 自动迁移、旧版本专用读取器和格式回退 | 当前维护边界不提供此能力 | 当前 schema、构建声明和 API 直接生效；旧数据停服备份后按产品流程处理 |
| FND-391 | 自动HTTP 重试和修改操作幂等 | 每个业务调用方 | http-client fetch一次；产品operation测试 |
| FND-392 | 文件/媒体流 transport | 产品client | http-client只处理有界JSON |
| FND-393 | 产品业务页面、品牌、业务 store 和专有路由 | 各产品管理 Web | admin-shell/admin-ui 提供公共导航、登录、账户等外壳；产品业务页面独立验收 |
| FND-394 | 浏览器会话持久化 | 明确不实现 | 无local/sessionStorage/IndexedDB源码 |
| FND-395 | 产品独有 UI 组件、品牌资源与可视化 | 各产品业务 Web | admin-ui/admin-shell/web-fonts 提供公共组件和字体；产品专有资源仍在产品 |
| FND-396 | 产品 Web 运行形态选择 | 产品清单声明 React 或原生 ESM | 消费者独立验收所选运行形态与业务页面 |
| FND-397 | 产品发行目录/mode/self-binding规则 | 各产品发行 verifier | xcss verifier后继续产品验证 |
| FND-398 | 产品 telemetry 出口、业务指标与外部 exporter 策略 | 各产品运行层 | log/server_runtime 已提供公共日志和运行机制；产品决定业务指标与外部出口 |
| FND-399 | 服务端安装、systemd、反向代理和运行配置 | 各产品`deploy/`/`config/` | xcss无deploy/config；消费者运维验证 |

## 组件依赖与责任图

```text
xcss::admin_auth ───────────────┐
                               ├─> xcss::contracts
xcss::error ────────────────────┤
xcss::schema_identity ──────────┘
       └─> xcss::sqlite

@xcss/web/contracts ──> @xcss/web/http-client ──┐
                                          ├─> @xcss/web/admin-web
React/Vite peers ─────────────────────────┘

@xcss/web/design-tokens（同包子路径，按需导入）
xcss::server_target（目标常量；整个 xcss crate 另有根编译硬门禁）
```

依赖箭头不转移产品责任。例如 `xcss::contracts` 依赖 admin-auth 只是复用规范的 username/token 验证器，
并不让 contracts 模块拥有业务数据库；管理员持久化由同一 crate 的 admin_sqlite 等模块提供。
`admin-web` 的网络依赖不改变服务端 Cookie 的所有权，Cookie 规则由公共 HTTP 适配器固定。
以上只是部分依赖关系，不表示省略的模块不存在；完整模块和职责见入门指南的组件地图。

## 关键取舍矩阵

| 选择 | 得到的收益 | 明确付出的成本 | 何时重新评审 |
|---|---|---|---|
| 构建期而非中央service | 生产故障域独立、断网运行 | 每个产品都要显式升级重建 | 只有出现不可编入产品的真实共同能力 |
| 仅支持当前格式精确的合同 | 漂移立即失败、边界可证明 | 破坏性变更需同步所有消费者 | 不使用宽松兼容或历史格式转换 |
| 单一admin角色 | 授权面、Schema、UI和审计最小 | 不提供只读/操作员管理账户 | 出现符合公共职责的真实分权需求并具备完整威胁模型时 |
| 精确Argon2 规则 | 启动/登录成本和状态唯一 | 参数变化需显式更新当前凭据 | 安全基线变化时发布新当前版本 |
| 严格Origin/Host/Sec-Fetch-Site | 代理歧义和CSRF fail closed | 非浏览器脚本不能伪装管理页面 | 另建明确机器API，不加header 回退 |
| 只支持AMD64 GNU/Linux 服务端 | 部署、CI、ELF和运行假设一致 | 不提供ARM/musl 服务端 | 补齐全产品等价构建/部署/安全矩阵后 |
| Xczs 的 React 外壳组合原生业务模块 | 共享登录、导航与管理基线，同时保留文件业务实现 | 必须明确 DOM 所有权和资源构建边界 | 产品业务重构或公共外壳接口变化时 |
| 精确的 React/Vite版本 | 所有当前 React 管理 Web 构建可复核 | 工具链升级需锁步 | 独立大问题验证所有消费者后 |
| 纯Schema算法+SQLx 适配器 | rusqlite/SQLx共享且无native link冲突 | 产品仍写少量driver映射 | 新driver出现时添加适配器而非复制算法 |
| 不自动重试 | 不重复未知副作用 | 产品必须实现幂等/operation策略 | 仅在业务层有明确可重试操作时 |
| 有界缓冲的 JSON | API简单且防无界内存 | 不适合大文件/媒体流 | 使用产品专用流 transport |
| 限定作用域的 reset | 浏览器基线共享且不污染宿主 | 每个App需加作用域 attribute | 不改为全局reset |
| Git rev/release tgz消费 | 无需公共软件包注册中心且来源不可变 | rev/URL与lock更新更显式 | 若采用可信软件包注册中心来源记录再评审 |



## 当前单体公共机制补充

| ID | 实际实现 | 权威锚点与验证边界 |
|---|---|---|
| FND-410 | 分层具有类型约束的 JSON 配置、显式环境映射、叶字段来源与每层语义钩子 | `xcss::config`；结构/来源/秘密/输入预算测试；产品完成最终必需字段和外部前提检查 |
| FND-411 | 机器 CLI 错误、真实服务就绪状态身份核验、共同 HTTP 解析拒绝 | `xcss::server_cli`；单记录输出、HTTP parser/413/no-store、临时监听测试 |
| FND-412 | 同目录运行/维护/诊断写锁、持久维护门和通用维护描述符借用 | `xcss::state_file`；真实 flock、inode、pending、显式发行、持久维护门和真实owner/root权限测试 |
| FND-413 | 当前 WAL/journal 代的只读临时数据库校验副本 | `xcss::sqlite::validation_snapshot`；源字节不变、writer busy、query-only与clone 运行时校验测试；仅独立诊断进程，不声明备份 |
| FND-414 | 服务端结构化日志、公共事件模板、精确筛选和有界轮转 | `xcss::log`；UTC/脱敏/limits/query/真实rotation/tracing sink切换；GNU/Linux AMD64 原生验证与产品验收 |
| FND-415 | 既有静态管理员只读检查、只在首次初始化写当前账户文件 | `xcss::admin_static`；当前格式与准确configured IDs、不创建文件、不改持久字节 |

历史 v0.10.4 验收记录只证明其当时源码与状态，不作为当前单体的验收证据。当前正式版本、完整修订号、锁闭包、package/license 清单和产品发行物必须绑定同一实际源码并同步验收。API、适用边界和公共模块的实际消费入口见 [配置、CLI、锁与日志](../configuration-cli-logging.md)。
