# 数据库所有权

推荐部署布局：

```text
PostgreSQL cluster
├── sarmg_platform
│   ├── core
│   ├── sunshine
│   └── host_monitoring
├── sentinel_monitor      独立 database/role
└── photo_backup          独立 database/role

Dufs                      本地 SQLite，绑定共享文件根
Photo mobile agent        本地 SQLite，离线队列
```

规则：

1. 每个模块拥有自己的 migration 和写权限。
2. 禁止跨模块业务外键、跨模块 SQL 查询和共享业务表。
3. 平台身份只以不透明主体 ID 传递；模块不得连接平台 session 表。
4. 统一 PostgreSQL 集群只统一备份、监控、补丁和容量管理，不统一数据生命周期。
5. Dufs 默认保留 SQLite。只有出现明确多节点需求时，才在文件操作 fencing、幂等提交和
   网络分区语义设计完成后增加可选 PostgreSQL `StateStore`。

UnionC 的 SQLite 到 PostgreSQL 迁移必须在模块边界稳定后单独执行。SQLite 文件身份、WAL、
完整性检查和离线恢复不能简单删除，必须由 PostgreSQL migration lock、readiness、逻辑备份
与经过演练的数据导入替代。

