# 09. 供应链、安全事件与日常运维

## 9.1 为什么共享库事件影响更大

xcss代码会被编译进多个服务端和Web，一个认证或软件包漏洞不会表现为“xcss服务宕机”，而
会静默存在于每个已发布消费者。因此运维重点是不可变来源、消费者可达性、重建证据和逐产品发布。

## 9.2 威胁面地图

```text
源码与Git
├─ commit/tag被替换
├─ 未提交文件混入release
└─ 恶意依赖/贡献

CI
├─ 浮动action/runner/toolchain
├─ 过大token权限
├─ checkout credential残留
└─ 不可信PR获取release权限

Package/Release
├─ stale dist或workspace-only依赖
├─ tar path/link/special file
├─ asset被替换
└─ verifier TOCTOU/资源耗尽

消费者
├─ 永久path/file依赖
├─ lock未更新
├─ 更强产品边界被删除
└─ 已编译旧制品未重发
```

## 9.3 Git与标签

发行要求完整40位源码修订号、工作树无tracked/untracked变化、唯一`v1.0.0` 标签指向HEAD。annotated
标签和发行一旦公开不移动或覆盖。若发现问题，保留证据并发新版本。

GitHub organization名、域名和部署命名空间不等于项目旧名称；身份变更必须区分项目名与组织所有者，
避免误删合法remote/邮箱。

## 9.4 Action与权限

所有action使用allowlist中的完整commit SHA，runner固定ubuntu-24.04，job有1～30分钟超时。顶层权限空，
普通job只有contents read；源码检出不持久凭据。只有tag-only 发行 job有contents write。

YAML anchor/alias/merge被禁，是因为文本和解析结构间差异可能绕过简单规则。新增workflow特性时要同时写
恶意测试夹具证明检查仍fail-closed。

## 9.5 Registry与软件包

当前主要用GitHub 发行 tgz，而不是假设公共软件包注册中心永远可用。若使用npm 可信 publishing/provenance，
权限仍应最小化，Token不进入本地文件、log、artifact或support 构建资源包。

软件包发行前审查真实tgz，而不是只看src。离线空目录安装证明依赖完整；`--ignore-scripts`减少安装时
第三方执行面。

Rust crate 也不能只在工作区根声明 `license="Apache-2.0"`。`cargo vendor` 会把每个 crate 展平为
独立软件包；若 crate 根没有自己的普通 `LICENSE`，消费者生成第三方 notices 时不能安全借用另一个项目的
通用文本。xcss 的单体 crate 因此携带审核过的根许可证，并用真实
`cargo package --list` 证明文件确实进入分发清单。

## 9.6 发行资产复核

下载后先核`SHA256SUMS`，再用发行 tool对artifacts与release-tree做精确的验证。两者职责不同：checksums
方便单文件校验，release-tree还证明文件集合、路径、mode和size没有额外/缺失。

build inventory记录工具链、lock hash、组件和asset；它不是签名，但为事故调查提供输入清单。平台托管的
发行 metadata/provenance应与仓库证据一起保存。

## 9.7 认证漏洞响应

若发现管理员 username 规范化、Argon2、令牌、同源、CSRF 或 admin-web 竞态问题：

1. 暂停xcss和所有消费者发行；
2. 确认受影响函数、版本和产品可达路径；
3. 检查是否已有Session/凭据需要产品级撤销或重建；
4. 修复唯一当前实现和攻击负例；
5. 发布新xcss不可变版本；
6. 每个产品更新依赖、重建、测试并发布；
7. 按产品运维策略撤销Session/轮换凭据；
8. 记录每个产品的修复源码、实际CI、正式发行身份和事件处理结果。

不能依赖“库已经修复”结束事件，因为生产binary不会自动变化。

## 9.8 密码规则变化

如果安全评审决定更改Argon2参数，新xcss版本只接受新当前规则。在线服务端不同时验证两套。
稳定环境需要离线重新设置/转换管理员凭据，由有明确权限和审计的独立流程执行；开发期数据直接重建。

不得把raw密码或hash放进日志、发行、测试夹具或issue。测试使用明显虚构密码和每次随机salt。

## 9.9 依赖漏洞响应

确认公告对应的版本、启用feature、可达代码和native/transitive影响。修复顺序：xcss依赖/lock→全
门禁→真实tgz→每个采用组件的消费者→产品发行。没有采用受影响组件的消费者不应仅因传递猜测列入，
产品当前源码、清单和锁文件帮助定位实际采用范围。

## 9.10 服务端编译目标事件

若发现非AMD64 服务端制品被构建或发布，检查：服务端 crate是否直接依赖target gate、产品build.rs、Cargo
target参数、发行身份、ELF machine、启动平台检查和CI matrix。不要把该制品“标尽力而为”；
当前合同明确不支持，应撤下并重建正确target。

客户端跨平台制品不属于该事件。先根据binary职责分类，避免误撤xsoc或移动客户端。

## 9.11 Secret边界

xcss可能接触的敏感物主要是CI/release 凭据，而不是产品Secret。产品管理员密码、会话令牌、
external key、数据库、摄像头URL等不应进入本仓。测试夹具必须使用`.test`域、确定无效的虚构key或随机临时值。

错误响应结构 details也不是内部诊断转储；raw上游body、SQL、PHC和令牌只在必要且脱敏的产品日志处理。

## 9.12 日常检查频率

每个发行周期：

- 固定工具链与lock clean install；
- 工作流校验规则和invalid 测试夹具；
- 唯一tgz的隔离install；
- deterministic asset/release-tree；
- 产品当前源码、CI和正式发行资产身份；
- 取消名称/仅支持当前格式扫描；
- 6个产品认证/服务端编译目标抽查；
- 中文文档API示例与软件包导出s抽查。

定期但不必每次commit：从GitHub回下载资产复核、软件包注册中心 ownership/credential审计、依赖公告、空缓存构建
和完整消费者发行演练。

## 9.13 备份

保存Git objects、annotated tags、发行 metadata/assets、workflow、lock、Schema/fixture、产品CI与正式发行验收结果和文档。最好有与GitHub/registry独立的备份位置。`node_modules`、`target`、dist和软件包注册中心 cache不能替代源码
或发行备份。

xcss没有业务数据库。State/Backup合同提供机器可验证的数据描述，不定义产品操作流程。

## 9.14 事件沟通

报告应包含受影响版本/组件、可达消费者、发现时间、证据hash、临时停止措施、修复commit/新版本、每个
产品重建状态和剩余风险。不要包含raw Secret或可利用的生产路径。无法确认的部分标为待验证，不用“应该
没问题”替代证据。

## 9.15 本章练习

1. 假设admin-web出现stale 401漏洞，列出从暂停发布到六个消费者重发的步骤。
2. 比较SHA256SUMS与release-tree能分别发现什么。
3. 评审一个使用`actions/checkout@v4`的workflow为何不合格。
4. 区分xcss发布Token、产品管理员密码和external key的所有者。
5. 为非AMD64 服务端 asset事件写证据清单，同时避免误判客户端。
