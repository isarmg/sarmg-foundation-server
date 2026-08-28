# 数据库所有权

## 推荐部署

```text
PostgreSQL cluster
├── sarmg_platform database
│   ├── sunshine schema         owner/runtime role: sunshine_runtime
│   └── host_monitoring schema  owner/runtime role: host_monitoring_runtime
├── sentinel_monitor database   role: sentinel_monitor_runtime
└── photo_backup database       role: photo_backup_runtime

<UNIONC_DATA_DIR>/modules/dufs/state/  Dufs SQLite 与共享根提交恢复
Photo mobile/agent local SQLite       离线队列，不属于服务器数据库
```

统一 PostgreSQL 指统一集群的补丁、备份、监控、连接治理和容量规划，不代表共享业务 schema、
role 或生命周期。

## 所有权矩阵

| 模块 | profile | 配置中心字段 | 独占身份 | 说明 |
|---|---|---|---|---|
| Sunshine | `postgres_schema` | `/database_url` | schema `sunshine`、role `sunshine_runtime` | 从 Union SQLite 导入后由 worker 独占写入 |
| 主机监控 | `postgres_schema` | `/database_url` | schema `host_monitoring`、role `host_monitoring_runtime` | Agent、遥测、历史和配对数据 |
| Sentinel | `dedicated_postgres` | `/database_url` | database/role `sentinel_monitor_runtime` | 摄像头、事件和审计 |
| Photo Backup | `dedicated_postgres` | `/database_url` | database/role `photo_backup_runtime` | 资产元数据；原文件在独立数据卷 |
| Dufs | `embedded_sqlite` | 无网络 URL | `modules/dufs/state` | 文件操作状态与共享根同故障域 |

配置先按模块 `config/schema.json` 校验，再写入 `UNION_PLUGIN_CONFIG` 指向的只读文件。旧 worker
需要 `DATABASE_URL` 时，只能通过 Manifest `environment.config_pointer` 显式映射；不得继承宿主或
其他模块的环境。

## 强制规则

1. 每个模块仓库拥有且只运行自己的 migration。
2. runtime role 只能写本模块 schema/database；migration role 可分离并应在启动前降权。
3. 禁止跨模块业务外键、跨模块 SQL、共享可写表和直接读取 Union session 表。
4. 平台身份通过不透明主体与短时内部证明传递，不通过数据库 join 传递。
5. 备份与恢复按模块故障域验证；“同一集群快照成功”不等于每个文件数据卷一致。
6. schema、role、service 与配置映射在运行时 catalog 内必须唯一且通过 Manifest 校验。

## 为什么 Dufs 保留 SQLite

Dufs 的 upload session、operation、purge job 与共享根 inode、rename、fsync 和崩溃恢复共同
组成一次提交协议。把状态远移到 PostgreSQL 会引入数据库提交成功但文件系统提交未知，以及
网络分区下 fencing 不明确的问题。当前单节点目标下，本地 SQLite 更容易建立可证明的一致性。

只有同时完成多节点锁、fencing token、幂等文件提交、网络分区恢复和真实故障注入后，才能
增加另一个 StateStore；不能为了“数据库统一”直接替换。

## SQLite 切换 PostgreSQL 的门禁

Sunshine/主机旧数据迁移必须：

1. 冻结旧写入口并取得 Union SQLite 的一致快照；
2. 由目标 worker 的 importer 读取、规范化并写入其独占 schema；
3. 记录来源 fingerprint、行数和可复验的导入批次；
4. 对目标逐行/聚合核验，保留精确 rollback journal；
5. 在候选发行 catalog 中切换模块版本后进行读写冒烟；
6. 只有回滚窗口结束后才移除旧表和密钥。

不能用双写长期维持两个事实源，也不能让两个进程同时持有旧 SQLite 独占状态。
