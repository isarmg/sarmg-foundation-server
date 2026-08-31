# 04. Contracts、Schema Identity、Guard 与 JSON Schema

## 4.1 四种东西不是一回事

| 层 | 何时工作 | 能保证什么 | 不能保证什么 |
|---|---|---|---|
| TypeScript type | 编译时 | 已经被正确标注的代码字段提示 | 网络JSON真的符合类型 |
| runtime guard | JavaScript运行时 | 一个`unknown`值当前是否符合合同 | 自动验证未调用它的值 |
| JSON Schema | validator运行时 | 非TS工具可按机器规则验证 | 任意validator配置都正确；文件存在即生效 |
| Rust serde + validate | 反序列化/显式验证 | Rust输入严格shape与语义 | 产品物理路径、Schema current值等加强规则 |

Fixture是第五层：用同一组valid/invalid JSON证明各实现对边界理解一致。

## 4.2 为什么从`unknown`开始

```ts
import { isAdministratorSession } from "@sarmg/contracts";

const candidate: unknown = JSON.parse(text);
if (!isAdministratorSession(candidate)) {
  throw new Error("管理员Session合同不匹配");
}
// 此处之后candidate才被narrow为AdministratorSession。
```

把网络值直接写成`const candidate: AdministratorSession = await response.json()`只改变编译器看法，不检查
任何字节。所有HTTP、文件、postMessage和外部工具输入都应先是`unknown`。

## 4.3 Exact keys

多数当前合同使用exact keys。假设Session多返回`permissions`：宽松parser可能忽略它，而另一个client可能
开始依赖它，造成同一版本内隐式协议分叉。strict guard会直接失败，迫使Server、Web、Schema、fixture和
版本一起改变。

Error Envelope的`request_id`/`details`是明确定义的optional fields，所以使用“required + allowed”集合；
其他未知字段仍拒绝。

## 4.4 管理员登录候选与权威验证

`AdministratorLoginRequest`只要求：

- exact `{username,password}`；
- username 为 1～64 个 printable ASCII bytes；
- password 为 1～1024 code points 且不得含 U+0000..001F 或 U+007F。

它故意不要求 username 已经 lowercase/canonical，也不在 Web 中实现 Argon2 plaintext byte policy。
比如 `admin@example.test` 作为有界 printable ASCII candidate 能通过这个 guard，但 Server 的 canonical
username admission 必须拒绝它。登录表单提交候选值，Server 的 `sarmg-admin-auth` 才是
trim/lower/canonical 和密码验收的权威层。合同通过≠凭据有效；JSON 字段只有 `username`，不存在
`email` alias。

## 4.5 AdministratorSession

Session固定五字段：

| 字段 | 规则 | 为什么严格 |
|---|---|---|
| `authenticated` | literal `true` | 不允许“匿名Session对象”进入已认证分支 |
| `user_id` | 1..128 ASCII identifier | 可安全用于日志、路由与关联 |
| `username` | 3～64 bytes canonical `[a-z0-9._-]`，首尾字母数字 | 所有产品展示/比较同一身份；不含邮箱语义 |
| `role` | literal `admin` | 管理面没有第二角色/兼容分支 |
| `csrf_token` | canonical 43字符URL-safe token | 与Server当前token primitive一致 |

Rust序列化前也调用`validate()`，避免手工构造`authenticated=false`后序列化。正常Server应使用
`AdministratorSession::new(user_id,username,csrf)`，构造器自动固定 admin。

## 4.6 Error Envelope

Error合同把machine与展示分开：`code`稳定、`message`可面向用户、`retryable`显式、`request_id`可选，
`details`为对象。客户端不能按中文/英文message分支。

需要注意：`request_id`可以缺失，但显式`null`无效；`details`缺失与空对象在序列化输出上可能都表现为
缺失，但解析时若存在就必须是object。

## 4.7 Safe integer

JavaScript只能精确表示到`9_007_199_254_740_991`。Rust的`u64`更大，但wire合同必须以所有消费者都能精确
理解的交集为准。因此revision、bytes、files、epoch seconds、envelope version等都使用custom serde
visitor限制safe integer，并拒绝负数、fraction和过大值。

这不是性能优化，而是防止如下静默变化：

```text
Rust发送 9007199254740993
JavaScript读取后得到 9007199254740992
```

## 4.8 State Contract

State Contract描述一个产品当前持久状态需要什么，不执行backup：

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
recordings/companion-contract。共享合同不要求资源name唯一或排序，因为产品组合规则不同；产品必须在guard
后加强。

## 4.9 Release Identity

Release Identity只有五字段：product、version、source_revision、target、state_contract_sha256。它不包含
upgrade tool version、历史edge或任意说明文本。state contract先写入，再对其exact bytes做SHA-256，最后
把hash写进identity；顺序反过来会失去绑定。

## 4.10 Backup Manifest

Backup Manifest v2描述：工具版本、产品、应用版本、可空完整SchemaIdentity、创建时间、外部Secret要求和
至少一个资源。每个资源有name/kind/path/bytes/files/SHA。

共享guard只要求path非空，不声称它安全、canonical或位于允许根；离线adapter仍要做no-follow、路径锚定、
mode/owner、实际bytes/files/hash和资源组合验证。Manifest合同不是backup实现。

## 4.11 完整SchemaIdentity与State内Schema的差异

完整`SchemaIdentity`有application、application_version、schema_revision、schema_sha256，用于数据库与
Backup。State Contract顶层已经有application/version，所以内嵌schema只需revision/sha256。不能为了“复用
一个struct”改变wire shape；Rust为二者使用不同类型是刻意设计。

## 4.12 JSON Schema公开入口

不要深层导入源码路径，使用package exports，例如：

```js
import administratorSchema from
  "@sarmg/contracts/schemas/administrator-auth.schema.json" with { type: "json" };
```

具体Node/bundler的JSON import语法由消费者环境决定。关键是解析公开export，而不是
`node_modules/@sarmg/contracts/dist/...`。发布smoke会检查5份Schema和5份fixture都在真实tgz中。

## 4.13 Fixture策略

每个合同fixture包含valid和invalid集合。invalid不仅包括类型错误，还包括：unknown field、missing field、
null区别、边界+1、uppercase hash、duplicate lock、空resources、错误role、非canonical token等。Rust测试
直接`include_str!`同一package fixture，防止复制后各自演进。

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
2. 同步TypeScript type、guard、JSON Schema；
3. 同步valid/invalid fixture；
4. 同步Rust struct/custom serde/validate；
5. 修改所有产品Server/client/tool调用；
6. 删除被替代field/reader/test/docs，不留alias；
7. 运行跨语言与真实产品release验证。

若字段变化涉及已发布持久manifest，由`sarmg-upgrade`建立精确离线转换；Foundation当前parser不同时接受
两个版本。

## 4.16 本章练习

1. 写一个TypeScript对象能通过编译但被Session guard拒绝的例子。
2. 列出State schema字段missing、null和object三种含义。
3. 构造一个超过safe integer的Backup bytes，说明Rust为何也应拒绝。
4. 在共享guard通过后，为一个产品设计三条更强resource规则。
5. 为新增字段写至少五类invalid fixture，而不仅是一个happy path。
