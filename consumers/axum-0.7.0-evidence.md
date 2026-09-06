# Axum 0.7.0 消费者验证证据

目标平台：`x86_64-unknown-linux-gnu`。Foundation 正式 revision：
`77e7ad7af8e1bf62432bd6bdd8fa9aff54cb39d1`，不可变版本 `v0.7.0`。

平台 [CI](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34037708626)
和 [Release](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34037886724)
均已通过，覆盖 Rust workspace、严格 Clippy、27 个 runtime 测试、Web 包、Python 37 项及发布树验证。
Rust 源码/清单/锁文件与完成消费者本地测试的候选
`99d5506c8146dfbe608e32441d97e51b924cdbb9` 完全相同；最终提交另外公开原生 Web 入口及可选 React peers。

## 消费者

精确产品 commit 见 `repositories.toml` 和生成的 `consumer-matrix.json`；不把历史 `baselines/` 改写成当前证据。

| 产品 | 已实际执行 | 远程验收 |
| --- | --- | --- |
| Sunshine Manager | workspace/all-targets 编译；`cargo test --lib`，31 通过；正式 0.7.0 npm 包构建 | [CI 通过](https://github.com/isarmg/sunshine-manager/actions/runs/34038340939) |
| Host Monitoring | workspace/all-targets 编译；`cargo test -p host-monitoring-server --lib`，44 通过；正式 npm 包构建 | [CI 通过](https://github.com/isarmg/host-monitoring/actions/runs/34038344244) |
| Sentinel Monitor | all-targets 编译；`cargo test --bin sentinel-monitor`，48 通过、1 个显式忽略；正式 npm 包构建 | [CI 通过](https://github.com/isarmg/sentinel-monitor/actions/runs/34038352976) |
| Media Backup | workspace/all-targets 编译；`cargo test -p media-backup-server --bin media-backup-server`，52 通过；正式 npm 包构建；更新当前 89 项 Web 资源合同 | [完整 CI 通过](https://github.com/isarmg/media-backup/actions/runs/34039177910)，含实际 server archive、Android 编译和未签名 iOS；不发布新的移动端版本 |
| Dufs RAM | 全量 all-targets/all-features Rust 635 通过、1 个显式忽略基准另行执行通过；原始 socket 合同；真实 release 硬期限非零退出和 SIGABRT 上传恢复通过；完整 Chromium/Firefox 浏览器矩阵及覆盖率通过 | [完整 CI 通过](https://github.com/isarmg/dufs-ram/actions/runs/34039807581)；本地 `./scripts/check.sh` 和正式签名包 E2E 复验中 |
| sarmg-upgrade | 单独提交支持边界，明确不支持 Dufs 0.51.0 状态备份/验证/恢复/升级 | 不冒充已实现新版本适配 |

本地构建使用 `CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_INCREMENTAL=0` 降低缓存占用，未修改断言、业务限额、release panic 策略或覆盖率门槛。临时测试根及状态与真实实例完全隔离。

## 正式平台制品

全部来自 [Foundation v0.7.0 Release](https://github.com/isarmg/sarmg-foundation-server/releases/tag/v0.7.0)，消费者 npm lock 另固定 SHA-512 integrity。

| 制品 | SHA-256 |
| --- | --- |
| release-tree.json | `1844a4c53f2c2e92ce02501a228fcbca74e91d639911118b93456dd54667617f` |
| sarmg-admin-shell-0.7.0.tgz | `d0e6703db8041924754033ff29d4be19384781fb7cfeba2f48d4fded249147bf` |
| sarmg-admin-ui-0.7.0.tgz | `4f8db2cd6b8a4e77efa558e6b32f6117b021e569a71c326b5c9b18794e085c29` |
| sarmg-web-fonts-0.7.0.tgz | `4b6822b6f8a542598e9c6a598b87aa79cbcb9e41bec06529f4319668649b8511` |

Dufs 正式制品摘要必须待实际发行验证后补录，当前不宣称完成 A14。

## 例外收窄

删除五个过时的 `local-platform-stack` 例外。四个控制平面仅保留源扫描器对 Composer 生成 SQL 的识别限制；Media Backup 另外保留嵌套 schema 目录的定位限制。Dufs 仅保留未知认证子路径的拒绝路由、测试 Cookie 常量被文本扫描误判的识别限制。不再豁免实际认证、Runtime、策略、自建 Schema 或多版本混用。具体理由及规则见各产品新的例外文件。
