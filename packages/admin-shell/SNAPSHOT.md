# 当前 Shell 发行与接入

共享 Shell 提供亮/暗图标切换、顶部导航和全宽内容；诊断入口、面板和专用前端解析器已移除。
认证 client、CSRF 和管理员合同由正式 Foundation 包提供，不在产品中复制。

四个控制平面消费者固定 0.7.0，Dufs 固定 0.7.1 的 React Profile。Dufs 的登录、导航和页面结构由 React 渲染，文件/上传控制器保留独占 DOM 区域和既有协议。
公共默认值、消费者覆盖和产品对象映射见 [工作区配置](../../docs/admin-workspace.md)。

消费者从不可变 Release tarball 安装 `@sarmg/admin-shell`、`@sarmg/admin-ui` 等包并固定 lockfile integrity，
只使用一套 Shell Context；不再生成 `clients/web/shell/` 快照，也不需要同级 Foundation checkout。
通用原生 Web 入口仍供选择 `web-embedded-native` 的其他消费者使用，不表示 Dufs 仍是原生页面。

本文件保留原文档路径以便查阅；旧快照分发方式不是当前构建流程。
实际验收见 [0.7.0 记录](../../consumers/axum-0.7.0-evidence.md)与 [0.7.1 记录](../../consumers/react-filesystem-0.7.1-evidence.md)。
