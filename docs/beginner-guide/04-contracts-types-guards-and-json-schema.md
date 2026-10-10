# 04. 合同、结构身份、运行时校验与 JSON Schema

## 4.1 四种东西不是一回事

| 层 | 何时工作 | 能保证什么 | 不能保证什么 |
|---|---|---|---|
| TypeScript 类型 | 编译时 | 已经被正确标注的代码字段提示 | 网络JSON真的符合类型 |
| 运行时校验 | JavaScript运行时 | 一个`unknown`值当前是否符合合同 | 自动验证未调用它的值 |
| JSON Schema | 验证器运行时 | 非TS工具可按机器规则验证 | 任意验证器配置都正确；文件存在即生效 |
| Rust serde + validate | 反序列化/显式验证 | Rust输入严格shape与语义 | 产品物理路径、Schema 当前值等加强规则 |

测试夹具是第五层：用同一组有效和无效 JSON证明各实现对边界理解一致。

## 4.2 为什么从`unknown`开始

```ts
import { isAdministratorSession } from "@xcss/web/contracts";

const candidate: unknown = JSON.parse(text);
if (!isAdministratorSession(candidate)) {
  throw new Error("管理员Session合同不匹配");
}
// 此处之后candidate才被narrow为AdministratorSession。
```

把网络值直接写成`const candidate: AdministratorSession = await response.json()`只改变编译器看法，不检查
任何字节。所有HTTP、文件、postMessage和外部工具输入都应先是`unknown`。

## 4.3 精确字段集合

多数当前合同使用精确的 keys。假设会话多返回`permissions`：宽松解析器可能忽略它，而另一个client可能
开始依赖它，造成同一版本内隐式协议分叉。严格的运行时校验会直接失败，迫使服务端、Web、Schema、测试夹具和
版本一起改变。

错误响应结构的`request_id`/`details`是明确定义的可选字段，所以使用“required + allowed”集合；
其他未知字段仍拒绝。

## 4.4 管理员登录候选与权威验证

`AdministratorLoginRequest`只要求：

- 精确的 `{username,password}`；
- username 为 1～64 个可打印 ASCII 字节；
- password 为 1～1024 码点且不得含 U+0000..001F 或 U+007F。

它故意不要求 username 已经 lowercase/canonical，也不在 Web 中实现 Argon2 密码明文字节规则。
比如 `admin@example.test` 作为有界可打印 ASCII candidate 能通过这个运行时校验，但服务端的规范的
username 准入必须拒绝它。登录表单提交候选值，服务端的 `xcss::admin_auth` 才是
trim/lower/canonical 和密码验收的权威层。合同通过≠凭据有效；JSON 字段只有 `username`，不存在
`email` 别名。

## 4.5 管理员会话（AdministratorSession）

会话固定五字段：

| 字段 | 规则 | 为什么严格 |
|---|---|---|
| `authenticated` | literal `true` | 不允许“匿名会话对象”进入已认证分支 |
| `user_id` | 1..128 ASCII identifier | 可安全用于日志、路由与关联 |
| `username` | 3～64 字节规范的 `[a-z0-9._-]`，首尾字母数字 | 所有产品展示/比较同一身份；不含邮箱语义 |
| `role` | literal `admin` | 管理面没有第二角色/兼容分支 |
| `csrf_token` | 规范的 43字符URL 安全的令牌 | 与服务端当前令牌原语一致 |

Rust序列化前也调用`validate()`，避免手工构造`authenticated=false`后序列化。正常服务端应使用
`AdministratorSession::new(user_id,username,csrf)`，构造器自动固定 admin。

## 4.6 错误响应结构

Error合同把machine与展示分开：`code`稳定、`message`可面向用户、`retryable`显式、`request_id`可选，
`details`为对象。客户端不能按中文/英文message分支。

需要注意：`request_id`可以缺失，但显式`null`无效；`details`缺失与空对象在序列化输出上可能都表现为
缺失，但解析时若存在就必须是object。

## 4.7 安全整数

JavaScript只能精确表示到`9_007_199_254_740_991`。Rust的`u64`更大，但通信格式合同必须以所有消费者都能精确
理解的交集为准。因此修订号、字节、files、epoch seconds、envelope version等都使用custom serde
visitor限制安全整数，并拒绝负数、小数和过大值。

这不是性能优化，而是防止如下静默变化：

```text
Rust发送 9007199254740993
JavaScript读取后得到 9007199254740992
```

## 4.8 状态合同

状态合同描述一个产品当前持久状态需要什么，不执行backup：

```json
{
  "contract_version": 1,
  "application": "example-product",
  "application_version": "1.0.0",
  "source_revision": "40位commit",
  "schema": {"revision": 1, "sha256": "64位hash"},
  "maintenance_locks": ["example-maintenance"],
  "resources": [{"name": "database", "kind": "sqlite", "required": true}],
  "external_requirements": [],
  "companion_contracts": []
}
```

若产品无数据库，`schema`必须存在且为`null`，不能省略。资源kind只有sqlite/configuration/data-tree/
recordings/companion-contract。共享合同不要求资源name唯一或排序，因为产品组合规则不同；产品必须在运行时校验
后加强。

## 4.9 发行身份

发行身份只有五字段：product、version、source_revision、target、state_contract_sha256。它不包含
额外的操作策略字段或任意说明文本。状态合同先写入，再对其精确的字节做SHA-256，最后
把hash写进identity；顺序反过来会失去绑定。

## 4.10 备份清单

备份清单 v1描述：工具版本、产品、应用版本、可空完整SchemaIdentity、创建时间、外部Secret要求和
至少一个资源。每个资源有name/kind/path/bytes/files/SHA。

共享运行时校验只要求path非空，不声称它安全、规范的或位于允许根；离线适配器仍要做不跟随符号链接的、路径锚定、
mode/owner、实际bytes/files/hash和资源组合验证。Manifest合同不是backup实现。

## 4.11 完整 SchemaIdentity 与状态合同内结构身份的差异

完整`SchemaIdentity`有application、application_version、schema_revision、schema_sha256，用于数据库与
Backup。状态合同顶层已经有application/version，所以内嵌schema只需revision/sha256。不能为了“复用
一个struct”改变通信格式 shape；Rust为二者使用不同类型是刻意设计。

## 4.12 JSON Schema 公开入口

不要深层导入源码路径，使用软件包导出s，例如：

```js
import administratorSchema from
  "@xcss/web/contracts/schemas/administrator-auth.schema.json" with { type: "json" };
```

具体Node/bundler的JSON import语法由消费者环境决定。关键是解析公开导出入口，而不是
`node_modules/@xcss/web/dist/contracts/...`。发布冒烟验证会检查5份Schema和5份测试夹具都在真实tgz中。

## 4.13 测试夹具策略

每个合同测试夹具包含valid和invalid集合。invalid不仅包括类型错误，还包括：未知 field、缺失 field、
null区别、边界+1、大写 hash、重复 lock、空resources、错误role、非规范令牌等。Rust测试
直接`include_str!`同一软件包测试夹具，防止复制后各自演进。

## 4.14 产品加强示例

```text
共享isStateContract通过
├─ 产品检查application/version等于compiled current
├─ source revision与release一致
├─ maintenance lock集合/顺序符合产品合同
├─ resource name唯一且顺序canonical
├─ required资源完整
├─ companion platform/hash精确
└─ schema revision/hash等于产品当前值
```

共享层不能替代这些规则，因为不同产品的资源和锁天然不同。

## 4.15 合同变更步骤

1. 写新的唯一当前JSON示例与删除后果；
2. 同步TypeScript 类型、运行时校验、JSON Schema；
3. 同步valid/invalid 测试夹具；
4. 同步Rust struct/custom serde/validate；
5. 修改所有产品Server/client/tool调用；
6. 删除被替代field/reader/test/docs，不留别名；
7. 运行跨语言与真实产品发行验证。

## 4.16 本章练习

1. 写一个TypeScript对象能通过编译但被会话运行时校验拒绝的例子。
2. 列出State schema字段缺失、null和object三种含义。
3. 构造一个超过安全整数的Backup 字节，说明Rust为何也应拒绝。
4. 在共享运行时校验通过后，为一个产品设计三条更强resource规则。
5. 为新增字段写至少五类invalid 测试夹具，而不仅是一个happy path。
