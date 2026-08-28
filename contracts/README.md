# Contracts

`plugin-manifest-v1.schema.json` 是当前唯一模块清单 schema。旧 `module-v1.schema.json` 描述的是
已废止的 Cargo feature / private-process 静态模型，已删除且不能作为兼容入口。迁移必须显式生成
Plugin Manifest v1，不能只重命名旧文件。
