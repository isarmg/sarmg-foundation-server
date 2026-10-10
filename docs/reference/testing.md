# 专项测试参考

按本次改动选择相关场景，完整命令见[构建与测试](../development.md)。

## 如何设计负例

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

## 管理 Web竞态测试

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

## 合同测试

每个JSON 测试夹具应在TS 运行时校验与Rust 解析器运行。JSON Schema可由软件包测试确认结构/引用，真实消费者若
用Schema 验证器还需验证其配置：Draft版本、format是否启用、ref解析和未知 field。

特别关注JS 安全整数。Rust 解析器不能因字段类型是u64而接受超出JS精确范围的JSON。

## SQLite测试

使用临时目录和真实SQLite文件验证：existing 缺失、明确create、每连接PRAGMA、pool边界、foreign key
violation、完整性诊断、checkpoint busy/incomplete、元数据 DDL/column/storage class、0/2 row、每字段
当前 mismatch和实际结构指纹 drift。

文件安全、owner/mode 与实例锁分别由 `fs_safety`、`state_file` 的真实测试保障，不属于 `sqlite` 模块
单测；产品仍须验证业务资源授权、部署权限以及业务 backup crash recovery。

## 软件包与发行安全测试

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
