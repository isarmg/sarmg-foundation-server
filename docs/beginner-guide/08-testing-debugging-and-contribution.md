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
├─ 真实web bundle
├─ release/install/start
└─ 产品specific安全规则
```

每层证明不同事实，不能用更多单元测试替代产品发行测试。

## 8.2 如何设计负例

对任何外部输入至少考虑：

- 缺失、空值、null、wrong type、未知 field；
- 最小-1、最大、最大+1、非整数、Unicode/byte长度差异；
- 重复、不同顺序、逗号合并、大小写和非规范的等价表示；
- 控制字符、用户信息、path 遍历、symlink/hardlink；
- 超时、cancel、partial read、body增长和资源上限；
- 并发顺序、stale response、TOCTOU替换和crash中间点；
- 消费者更强规则是否仍然存在。

一个happy path和一个“明显错误字符串”不能证明严格的合同。

### 用户名、密码与散列

- username candidate 1/64/65 可打印 ASCII 字节，trim/lowercase；持久非规范的拒绝；
- 规范的 3/64/65 字节、`[a-z0-9._-]`、首尾字母数字；相邻分隔符允许；
- Unicode、控制字符、`@`、`+`、首尾点/下划线/连字符拒绝；
- 密码明文 11/12/1024/1025 字节与ASCII 控制字符；
- Argon2 algorithm/version/m/t/p/salt/output任一漂移；
- 规范的 PHC 往返一致性；启动时遍历全部持久管理员。

### 令牌与 Cookie

- 32-byte随机编码后恰43字符；
- 非规范的末尾bits、填充、wrong alphabet；
- expected digest 31/32/33 字节；
- 重复 cookie name、空value、非法name、多Cookie header line；
- 错误日志不包含真实 username/hash/token。

### 同源校验/CSRF

- Origin、Host、Sec-Fetch-Site 各自缺失、重复、含逗号或控制字符；
- Host与URI authority一致、只有authority、二者冲突；
- production HTTP、dev非回环地址、协议方案 mismatch；
- DNS case、default/explicit port、IPv4/IPv6 规范的；
- URL path/query/fragment/userinfo/opaque 来源；
- CSRF 缺失、重复、不符合规范或不匹配。

产品集成必须从真实framework HeaderMap/Request构造，不能只测xcss 字节 slice函数。

## 8.4 管理 Web竞态测试

用可控Promise/fake fetch精确安排：

1. 两次登录重叠，第一成功晚于第二调用；
2. 登录后立即退出，检查CSRF与修改操作顺序；
3. 恢复会话进行中退出；
4. 旧会话业务请求延迟401，新登录已成功；
5. 多组件并发恢复会话只fetch一次；
6. React 钩子卸载或client替换后旧Promise完成；
7. 退出网络失败时 UI 使用 `anonymous_logout_unconfirmed`，关闭本地授权并保留私有撤销快照供主动重试；
8. 运行时校验失败不会发布会话。

只用同步mock会漏掉该软件包最重要的价值。

## 8.5 合同测试

每个JSON 测试夹具应在TS 运行时校验与Rust 解析器运行。JSON Schema可由软件包测试确认结构/引用，真实消费者若
用Schema 验证器还需验证其配置：Draft版本、format是否启用、ref解析和未知 field。

特别关注JS 安全整数。Rust 解析器不能因字段类型是u64而接受超出JS精确范围的JSON。

## 8.6 SQLite测试

使用临时目录和真实SQLite文件验证：existing 缺失、明确create、每连接PRAGMA、pool边界、foreign key
violation、完整性诊断、checkpoint busy/incomplete、元数据 DDL/column/storage class、0/2 row、每字段
当前 mismatch和实际结构指纹 drift。

文件安全、owner/mode 与实例锁分别由 `fs_safety`、`state_file` 的真实测试保障，不属于 `sqlite` 模块
单测；产品仍须验证业务资源授权、部署权限以及业务 backup crash recovery。

## 8.7 软件包与发行安全测试

安全verifier必须证明fail-closed：

- linked dist/manifest/export；
- tar absolute/parent/backslash/duplicate/link/special member；
- 软件包意外src/test与缺导出入口；
- offline peer解析；
- 发行 path/mode/size/hash/extra/missing drift；
- hash过程中替换inode、增长或改变mtime；
- tree/file/count/manifest上限在超限时尽早失败；
- output指向root/nonempty/symlink；
- wrong tag/dirty tree；
- workflow浮动action、权限、凭据、YAML anchor。

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

先找到层，不要在产品加path hack或在xcss放宽合同掩盖真实错误。

## 8.9 新共享能力提案模板

写清：

1. 产品中立职责、实际消费者的仓库、文件与测试；暂时只有一个消费者也可以共享真正通用的机制；
2. 重复实现和真正相同的语义；
3. 差异及为何仍能抽最小原语；
4. 不可信输入、上限、编码和规范的规则；
5. output、具有类型约束的 error、retry/side-effect语义；
6. 状态、锁、Secret和权限所有者；
7. 最强消费者规则与产品保留规则；
8. 正例、边界、攻击、竞态和发行测试；
9. 版本、删除后果、消费者迁移和退出方案；
10. 为何不需要兼容层或在线service。

无法回答职责或语义边界时先留在产品，补充实际调用、安全约束和测试证据；不以第二个消费者作为机械准入条件。

## 8.10 代码评审问题

- 新public API是否比需要更大？
- 一个函数名是否暗示它保证了实际未验证的路径/权限/业务语义？
- 所有raw header line、字节和未知字段是否保留到严格的边界？
- 是否误把通信格式候选验证当认证成功？
- 是否把服务端编译目标依赖传播到客户端？
- 是否引入第二角色、旧字段、回退或隐式create？
- Web是否把令牌写到storage或接受跨源？
- 竞态中较旧response能否覆盖新state？
- 软件包测试是否从dist/tgz而非src/工作区验证？
- 产品实际验收是否绑定同一源码 commit、完整依赖身份和正式发行资产？

## 8.11 提交边界

一个大问题一个提交。认证primitive/合同、Web基线、服务端编译目标、SQLite identity、发布供应链和文档可以
分别形成逻辑提交；每个提交说明删除后果和定向验证。最终再统一跑全套并按仓库推送。

不要提交`target`、`node_modules`、本地dist、临时tgz、test-results、真实DB/Secret。锁文件、Schema、测试夹具
是受审事实源，必须在相应变更提交。

## 8.12 本章练习

1. 为`Sec-Fetch-Site`设计missing/duplicate/joined/value四组负例。
2. 写一个admin-web stale 401竞态时间线和断言。
3. 为新增备份字段列出 TypeScript、Rust、结构、测试夹具、消费者和发行工具的修改点。
4. 构造发行 hash过程中inode替换的威胁模型。
5. 评审一个“共享登录限流器”提案，指出哪些产品状态使它不适合直接共享。
