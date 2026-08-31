# 08. 测试、调试、代码评审与新增共享能力

## 8.1 测试金字塔按“边界”而非语言划分

```text
primitive单元测试
├─ 输入边界、typed error、算法golden
├─ auth/header/cookie/token攻击形状
└─ HTTP/React竞态

跨语言合同测试
├─ Rust serde/validate
├─ TypeScript guard
├─ JSON Schema
└─ 同一fixture

发布形态测试
├─ dist/export/tgz/offline install
├─ deterministic asset
└─ release-tree/TOCTOU/workflow权限

产品集成测试
├─ 真实router/DB/Cookie/proxy
├─ 真实clients/web bundle
├─ release/install/start
└─ 产品specific安全规则
```

每层证明不同事实，不能用更多单元测试替代产品release测试。

## 8.2 如何设计负例

对任何外部输入至少考虑：

- missing、empty、null、wrong type、unknown field；
- 最小-1、最大、最大+1、非整数、Unicode/byte长度差异；
- duplicate、不同顺序、逗号合并、大小写和非canonical等价表示；
- 控制字符、userinfo、path traversal、symlink/hardlink；
- timeout、cancel、partial read、body增长和资源上限；
- 并发顺序、stale response、TOCTOU替换和crash中间点；
- consumer更强规则是否仍然存在。

一个happy path和一个“明显错误字符串”不能证明strict合同。

## 8.3 管理员认证测试清单

### Username/password/hash

- username candidate 1/64/65 printable ASCII bytes，trim/lowercase；持久非 canonical 拒绝；
- canonical 3/64/65 bytes、`[a-z0-9._-]`、首尾字母数字；相邻分隔符允许；
- Unicode、control、`@`、`+`、首尾点/下划线/连字符拒绝；
- plaintext 11/12/1024/1025 bytes与ASCII control；
- Argon2 algorithm/version/m/t/p/salt/output任一漂移；
- canonical PHC round-trip；启动时遍历全部持久管理员。

### Token/Cookie

- 32-byte随机编码后恰43字符；
- 非canonical末尾bits、padding、wrong alphabet；
- expected digest 31/32/33 bytes；
- duplicate cookie name、空value、非法name、多Cookie header line；
- 错误日志不包含真实 username/hash/token。

### Same-origin/CSRF

- Origin/Host/Sec-Fetch-Site各自missing/duplicate/comma/control；
- Host与URI authority一致、只有authority、二者冲突；
- production HTTP、dev非loopback、scheme mismatch；
- DNS case、default/explicit port、IPv4/IPv6 canonical；
- URL path/query/fragment/userinfo/opaque origin；
- CSRF missing/duplicate/noncanonical/mismatch。

产品集成必须从真实framework HeaderMap/Request构造，不能只测Foundation byte slice函数。

## 8.4 Admin Web竞态测试

用可控Promise/fake fetch精确安排：

1. 两次login重叠，第一成功晚于第二调用；
2. login后立即logout，检查CSRF与mutation顺序；
3. restore进行中logout；
4. 旧Session业务请求延迟401，新login已成功；
5. 多组件并发restore只fetch一次；
6. React hook卸载或client替换后旧Promise完成；
7. logout网络失败但UI仍anonymous且private snapshot清理；
8. guard失败不会发布Session。

只用同步mock会漏掉该package最重要的价值。

## 8.5 Contracts测试

每个JSON fixture应在TS guard与Rust parser运行。JSON Schema可由package测试确认结构/引用，真实消费者若
用Schema validator还需验证其配置：Draft版本、format是否启用、ref解析和unknown field。

特别关注JS safe integer。Rust parser不能因字段类型是u64而接受超出JS精确范围的JSON。

## 8.6 SQLite测试

使用临时目录和真实SQLite文件验证：existing missing、明确create、每连接PRAGMA、pool边界、foreign key
violation、integrity诊断、checkpoint busy/incomplete、metadata DDL/column/storage class、0/2 row、每字段
current mismatch和实际fingerprint drift。

路径no-follow、owner/mode、实例锁和backup crash recovery不属于`sarmg-sqlite`单测，应在产品层测试。

## 8.7 Package与release安全测试

安全verifier必须证明fail-closed：

- linked dist/manifest/export；
- tar absolute/parent/backslash/duplicate/link/special member；
- package意外src/test与缺export；
- offline peer解析；
- release path/mode/size/hash/extra/missing drift；
- hash过程中替换inode、增长或改变mtime；
- tree/file/count/manifest上限在超限时尽早失败；
- output指向root/nonempty/symlink；
- wrong tag/dirty tree；
- workflow浮动action、权限、credential、YAML anchor。

## 8.8 调试定位树

```text
失败
├─ policy在build前失败
│  ├─ version/member/toolchain/依赖 -> manifest与policy事实源
│  └─ workflow权限/action -> workflow与invalid fixture
├─ Rust编译/测试失败
│  ├─ public API/type -> crate调用和consumer adapter
│  ├─ target compile_error -> 是否错误把Server gate给了client
│  └─ SQLx/identity -> driver边界与测试DB
├─ TypeScript失败
│  ├─ source type -> package/consumer调用
│  ├─ toolchain assertion -> exact manifest/.node-version
│  └─ React hook -> async generation/client lifetime
├─ package smoke失败
│  ├─ dist缺失/stale -> clean/build/copy
│  ├─ export/tar -> manifest/inventory
│  └─ offline peer -> package依赖所有权
└─ consumer失败
   ├─ shared合同过强/过弱 -> 回到准入设计
   ├─ 产品adapter错误 -> 保留产品加强规则
   └─ sibling path假成功 -> immutable rev/tgz独立checkout
```

先找到层，不要在产品加path hack或在Foundation放宽合同掩盖真实错误。

## 8.9 新共享能力提案模板

写清：

1. 两个以上当前消费者的仓库、文件、测试；
2. 重复实现和真正相同的语义；
3. 差异及为何仍能抽最小primitive；
4. 不可信输入、上限、编码和canonical规则；
5. output、typed error、retry/side-effect语义；
6. 状态、锁、Secret和权限所有者；
7. 最强消费者规则与产品保留规则；
8. 正例、边界、攻击、竞态和release测试；
9. 版本、删除后果、consumer迁移和退出方案；
10. 为何不需要兼容层或在线service。

无法回答时先留在产品，收集第二消费者证据。

## 8.10 代码评审问题

- 新public API是否比需要更大？
- 一个函数名是否暗示它保证了实际未验证的路径/权限/业务语义？
- 所有raw header line、bytes和unknown字段是否保留到strict边界？
- 是否误把wire候选validation当认证成功？
- 是否把Server target依赖传播到客户端？
- 是否引入第二角色、旧字段、fallback或隐式create？
- Web是否把token写到storage或接受cross-origin？
- 竞态中较旧response能否覆盖新state？
- package测试是否从dist/tgz而非src/workspace验证？
- consumer matrix证据是否真实？

## 8.11 提交边界

一个大问题一个提交。认证primitive/合同、Web基线、Server target、SQLite identity、发布供应链和文档可以
分别形成逻辑提交；每个提交说明删除后果和定向验证。最终再统一跑全套并按仓库推送。

不要提交`target`、`node_modules`、本地dist、临时tgz、test-results、真实DB/Secret。lockfile、Schema、fixture
是受审事实源，必须在相应变更提交。

## 8.12 本章练习

1. 为`Sec-Fetch-Site`设计missing/duplicate/joined/value四组负例。
2. 写一个admin-web stale 401竞态时间线和断言。
3. 为新增Backup字段列出TS/Rust/Schema/fixture/consumer/release修改点。
4. 构造release hash过程中inode替换的威胁模型。
5. 评审一个“共享登录限流器”提案，指出哪些产品状态使它不适合直接共享。
