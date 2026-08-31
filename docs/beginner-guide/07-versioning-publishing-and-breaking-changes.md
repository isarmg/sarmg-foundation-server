# 07. 版本、发布与破坏性变更

## 7.1 当前阶段

组件当前为 `0.2.0`。0.x 允许破坏性演进，但不允许不记录、不测试的漂移。一个发布版本一旦公开即不可
覆盖，同版本资产必须字节稳定。

## 7.2 Breaking change

删除/重命名 export、改变 guard/schema、错误语义、默认 timeout、SQLite pragma 或 CSS token 都可能破坏
消费者。先列真实消费者和迁移动作，再修改当前 API。

## 7.3 无兼容策略

新版本只提供新唯一名称/合同：删除 deprecated wrapper、别名、re-export、dual schema 和旧 CSS property。
消费者显式升级。历史数据转换不属于此无状态仓库。

## 7.4 发布顺序

```text
design/API review -> code+tests+docs -> version+locks
 -> Foundation full gates -> package content inspection
 -> each consumer pin/update/full CI -> immutable publish
```

## 7.5 Crate 发布

检查 Cargo metadata、license/readme/files、依赖版本、feature 与打包内容。消费者不能依赖工作区偶然可见的
未声明 crate。发布后用 registry 形态的临时消费者验证。

## 7.6 npm 发布

检查 `exports`、`types`、ESM、dist、JSON Schema/CSS 静态文件和依赖。测试从 tarball/package export 导入，
不能只用 workspace symlink。

## 7.7 消费者升级

Foundation test 通过只证明通用合同。每个产品还要跑自己的协议、数据库、UI、安全和发行测试，并确认
生产制品无 runtime Foundation 依赖。

## 7.8 回退

包发布不可删除覆盖。若新版本有缺陷，发布新的修复版本；消费者可在源码依赖层回退并重建自身制品，
但不能让一个运行时同时加载两代 API 做兼容。

## 7.9 发布证据

保存 revision/tag、锁文件、工具链、质量门、package inventory、checksum 和消费者 CI，不保存 registry
Token 或产品 Secret。
