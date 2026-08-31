# 04. Contracts 类型、Guard 与 JSON Schema

## 4.1 三个层次

TypeScript 类型帮助编译期；runtime guard 验证运行时 `unknown`；JSON Schema 服务跨语言/工具验证。三者
语义应一致，但用途不同，不能只更新其中一个。

## 4.2 Exact keys

当前 guard 和 Schema 拒绝 unknown fields，避免调用方以为某字段有效而消费者忽略。新增/删除字段是
显式破坏性变更，不使用 `additionalProperties` 宽松接纳未来内容。

## 4.3 State contract

状态合同包含产品、当前版本、40 位小写源码 revision、Schema identity、资源、external Secret requirement
和 companion。每项有严格形状；它描述状态代，不执行备份或恢复。

## 4.4 Release contract

release schema 描述不可变制品身份、revision、target、API/Schema 和文件集合。消费者仍须自己验证路径、
Hash、mode、链接和业务身份；JSON Schema 通过不等于发行树可信。

## 4.5 Backup manifest

manifest schema 约束备份元数据和资源描述，不负责打开 SQLite、认证密文或计算 tree identity。升级工具
在 Schema 解析之后执行更强的 code allowlist 与物理验证。

## 4.6 Error envelope

Web contract 与 Rust error primitive 语义对齐，但不是自动代码生成关系。更改时需同时验证跨语言 fixture，
避免一端接受另一端拒绝。

## 4.7 Guard 写法

先验证普通对象/非数组，再验证 exact key set、每字段基本类型、格式与嵌套集合上限。不要用 truthiness
代替类型，也不要让 `null`、缺失和空字符串混为一谈。

## 4.8 Schema 变更流程

更新 TS type、guard、JSON Schema、正负 fixture、package version、锁文件与所有消费者；删除旧 export/
字段。若历史 manifest 需要解析，只能由相应离线工具的精确 adapter 拥有。

## 4.9 练习

给一个合法 state contract 分别加入 unknown key、错误 revision 大小写、非法 SHA、数组代对象和 null，
确认 guard 与 JSON Schema 都拒绝且没有抛出未处理异常。
