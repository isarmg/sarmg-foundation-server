# 01. 项目定位、边界与目录

## 1.1 一句话定位

Sarmg Foundation 是构建时共享库仓库，不是在线基础服务。产品锁定精确 crate/package 版本并把代码
编入自身制品；生产运行不访问本仓库、registry 或 CDN。

## 1.2 何时适合共享

至少两个真实产品已经拥有相同需求、语义、安全边界与失败模型，且可设计出明显小于产品业务的稳定
API。仅代码长得相似、预计以后会用、或为了统一命名，都不是充分理由。

## 1.3 当前结构

```text
rust/crates/sarmg-error       error wire primitives
rust/crates/sarmg-sqlite      SQLx/SQLite connection baseline
packages/contracts        TS types、runtime guards、JSON Schema
packages/http-client      bounded JSON HTTP helper
packages/design-tokens    TypeScript/CSS visual tokens
```

## 1.4 Foundation 不拥有

产品身份、业务数据库、路由、认证策略、运行锁、migration、backup、restore、Secret、UI shell 和部署均由
消费者拥有。共享库不能用弱默认覆盖产品更强约束。

## 1.5 依赖方向

产品依赖 Foundation 的精确版本；Foundation 不反向依赖产品，也不通过网络运行时回调消费者。Web package
之间显式声明依赖，不利用 re-export 隐藏所有权。

## 1.6 当前名称

仓库唯一项目身份是 `sarmg-foundation`；Rust crate 使用 `sarmg-*`，npm scope 使用 `@sarmg/*`，CSS custom
property 使用 `--sarmg-*`。源码、锁文件、包 metadata、Schema 示例、测试和文档必须一致，不提供另一
命名入口。

## 1.7 小型共享层的取舍

- 少量产品 glue code 重复，换取故障域独立。
- build-time 版本升级需要逐产品重建，换取生产无中央依赖。
- exact guard 维护成本更高，换取不可信 JSON 可真实验证。
- SQLite helper 保持底层，消费者要写 lifecycle，换取不误判业务 Schema。

## 1.8 新手误解

TypeScript 类型不验证运行时 JSON；成功打开 SQLite 不证明路径/Schema 安全；统一 error envelope 不等于
统一所有产品错误码；design tokens 不是组件库；monorepo 统一版本不代表 API 永久兼容。

## 1.9 本章检查

能判断一个候选能力为何应留在产品，能说清发布后生产为何不依赖 Foundation，并能列出当前五个组件的
责任边界。
