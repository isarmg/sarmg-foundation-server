# React 文件服务 0.7.1 验收记录

目标：`x86_64-unknown-linux-gnu`。Foundation 正式版本为 0.7.1，revision 为
`466ef3b7e19a5eea07292d5eeda1d014b47e5c59`。
[CI](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34042339546) 和
[Release](https://github.com/isarmg/sarmg-foundation-server/actions/runs/34042566142) 均通过，含 38 项 Python 合同测试。

本版只扩展文件服务 Profile 的 Web 选择，并统一包版本。`git diff --exit-code 77e7ad7af8e1bf62432bd6bdd8fa9aff54cb39d1 466ef3b7e19a5eea07292d5eeda1d014b47e5c59 -- 'rust/**/src/**' 'packages/**/src/**'` 无差异。
四个控制平面消费者未受这次 Profile 扩展影响，继续使用已经完整验证的 0.7.0；其真实提交与 CI 见 [Axum 0.7.0 记录](axum-0.7.0-evidence.md)，不冒充它们已升级至 0.7.1。

## Dufs React 改造

用户追加授权以 React 覆盖原手册的原生页面范围。Dufs 使用 `web-react-admin`、React/React DOM 19.2.8、官方 Foundation tarball URL 与锁定的 SHA-512 integrity。只有一套 React 页面启动入口；旧原生工作区初始化、静态表单和 DOM 翻译器已移除。

React 负责登录、导航及页面结构，文件/上传控制器拥有独占 DOM 区域。新增浏览器用例证明主题更新不会重建运行中的上传行或重复发送 PUT。产品源码仍通过严格 JSDoc/TypeScript 与 AST 安全检查；React DOM 的生成 bundle 使用依赖完整性、同源 CSP、512 KiB 预算和真实浏览器验收，不豁免产品源码的 HTML 注入。

已执行：

- `npm run check:js`、`npm run check:types`、`npm run check:docs` 和前端单元测试通过。
- `cargo test --locked --target x86_64-unknown-linux-gnu --all-targets --all-features --no-fail-fast`：635 通过、0 失败、1 个显式忽略基准。
- 单独执行 `cargo test --locked --target x86_64-unknown-linux-gnu --test pagination benchmark_one_hundred_thousand_entries -- --ignored --nocapture`：通过，十万项首屏 4.087200625 秒。单次结果不代表性能改进承诺。
- React 提交 `690c82c44dfc3edc0a1b4022d8b263fe88fa7bd8` 的[完整 CI](https://github.com/isarmg/dufs-ram/actions/runs/34043831610) 通过，含 Chromium/Firefox 各 122 项、覆盖率、部署及 release 运行检查。
- 随后将类型声明构建移入私有临时目录；`160e57eac6d420b37d7196fe1c987d902702078d` 的[完整 CI](https://github.com/isarmg/dufs-ram/actions/runs/34044451492) 通过。
- 本地总检查通过：635 项 Rust、82.11% 行覆盖率、Chromium/Firefox 各 122 项、release 运行时、部署及审计。该次运行中只调整了类型声明的生成目录，未修改运行时源码；不把它冒充全程固定在同一提交的运行。
- 最终干净提交 `39aa39680eca6ce5632bdeac26cbf5153cbb0d90` 另行执行本地 `./scripts/check.sh`，全程保持该源码提交，全部通过，结束时工作区干净。635 项 Rust、82.11% 行覆盖率、Chromium/Firefox 各 122 项、部署、实际 release 运行时及审计均通过。使用 `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 CARGO_BUILD_JOBS=2 DUFS_REQUIRE_SHELLCHECK=1`；仅降低开发缓存开销，不改变 release panic 政策、断言或门槛。未安装的 Microsoft Edge 是明确跳过项，不冒充已验证浏览器。
- 同一精确提交、`v0.51.0` tag 的[完整 CI](https://github.com/isarmg/dufs-ram/actions/runs/34045648644)、[依赖审计](https://github.com/isarmg/dufs-ram/actions/runs/34045648622)、[独立性门禁](https://github.com/isarmg/dufs-ram/actions/runs/34045648636)、[正式签名包 E2E](https://github.com/isarmg/dufs-ram/actions/runs/34045648661)及[发布工作流](https://github.com/isarmg/dufs-ram/actions/runs/34045648649)全部通过。正式包 E2E 实际独立验签、解包并验证 release 硬期限及 SIGABRT 上传恢复。

前一原生候选的正式包 E2E 因宿主夹具编译前缺少 Web 资源而失败；已补上夹具所在源码目录的资源构建。修复后的 React 提交 `690c82c44dfc3edc0a1b4022d8b263fe88fa7bd8` 的[正式包 E2E](https://github.com/isarmg/dufs-ram/actions/runs/34043832043) 已通过：临时测试密钥验签、独立解包、实际 release 硬期限非零退出/保留提交/不关闭状态，以及 SIGABRT 后上传状态恢复。该成功不改写旧失败历史；最终标签另行执行同一门禁。

## 手册最终验收对应

| 项目 | 证据与边界 |
| --- | --- |
| A1–A3 | 唯一 Axum 路由、Foundation Axum 认证、静态管理员及内存 Session；认证/HTTP fixture 和实际 socket 测试通过，无 Hyper Adapter 或旧总分派器 |
| A4–A6 | 流式 PUT/PATCH/下载、一次解码及 RootedFs；上传长度/检查点、原始路径、HEAD/Range/条件请求回归通过 |
| A7–A9 | Foundation 配额和分阶段停机；真实多监听器、慢客户端、阻塞工作、SIGTERM、提交不确定性及 release 硬期限测试通过 |
| A10–A12 | 真实部署和健康探针、唯一 React 资源及不可变依赖、严格 SchemaIdentity 回归通过；没有平台源码副本或旧状态兼容入口 |
| A13 | 精确 tag 的完整 CI 已通过；socket、故障注入、实际 release 运行时、Chromium/Firefox 各 122 项，不以纯 Handler 测试代替 |
| A14 | Foundation 正式制品和四个受影响控制平面消费者已有真实证据；Dufs 精确 tag 全部门禁、正式发布、独立下载 SHA-256 及内嵌源码版本核验通过，摘要如下 |

## 正式 Foundation 制品

来自 [v0.7.1 Release](https://github.com/isarmg/sarmg-foundation-server/releases/tag/v0.7.1)：

| 制品 | SHA-256 |
| --- | --- |
| release-tree.json | `05e02a2d9219d688ae1019041f0a0df07961fc6b020abf0d8e12ca900b867cca` |
| sarmg-admin-shell-0.7.1.tgz | `b7d4c2fcf55a1dad4207719749357301e234d343de3407ac38fd456c2330edf7` |
| sarmg-admin-ui-0.7.1.tgz | `dd9796d90dfd3af9ef56a3a1907e89b32f68ed9c57ba58a3d483926b429b5b88` |
| sarmg-web-fonts-0.7.1.tgz | `30f6c03a3aa35fbce4880a469b6fb5d797e863302841a02d8286baafad0da6aa` |

## 正式 Dufs 制品与独立核验

[Dufs v0.51.0](https://github.com/isarmg/dufs-ram/releases/tag/v0.51.0) 于 2026-09-06 17:08:33 UTC 正式发布，非草稿。tag 解引用为 `39aa39680eca6ce5632bdeac26cbf5153cbb0d90`，没有移动或覆盖旧 tag/制品。

| 下载项 | SHA-256 |
| --- | --- |
| dufs-0.51.0-x86_64-unknown-linux-gnu | `20db656dd4db59aaaa3dd87c36fd6afdfb27c892c41a4c85e20485ec6c10a406` |
| dufs-0.51.0-x86_64-unknown-linux-gnu.sha256 | `8481a316056ed67a136f0f11f2ad23ffb52819c032480b8cfcba0f24fd8869fb` |

发布后使用 `gh release download v0.51.0 --repo isarmg/dufs-ram` 下载到新的私有临时目录，执行 `sha256sum --check dufs-0.51.0-x86_64-unknown-linux-gnu.sha256` 成功；对两个下载项独立计算 SHA-256，与上表及 GitHub asset digest 一致。下载二进制的 `--version` 实际输出为 `dufs 0.51.0 (git 39aa39680eca6ce5632bdeac26cbf5153cbb0d90)`。

公开 Release 提供 Linux 二进制和校验文件；签名归档路径已通过临时测试密钥的 E2E，但不把公开下载项冒充已发布的签名归档。A1–A14 已完成本次手册约定的验收；消费者状态保留 `temporary-exception`，仅反映三项文本扫描误判的窄范围例外，不再是迁移未完成，也不伪称零例外。

sarmg-upgrade 仍明确不支持本版的备份、验证、恢复或升级，这是手册允许的明确支持边界，不是隐含适配。不允许修改旧 metadata 或把空库与旧共享树拼接；没有触碰真实实例数据。
