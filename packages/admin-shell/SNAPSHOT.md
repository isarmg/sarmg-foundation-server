# 管理 Shell 接入与验收

共享 Shell 提供亮/暗图标切换、顶部导航和全宽内容。认证客户端、CSRF 和管理员合同由正式 Foundation 包提供。
公共默认值与消费者覆盖方式见 [工作区配置](../../docs/admin-workspace.md)。

消费者从不可变 Release tarball 安装 `@sarmg/admin-shell`、`@sarmg/admin-ui` 等包并固定 lockfile integrity，
使用一套 Shell Context；独立构建不需要同级 Foundation checkout。
通用原生 Web 入口供选择 `web-embedded-native` 的消费者使用。

当前采用版本和状态见消费者矩阵。以下记录只证明各自版本的验收结果：

- [0.7.0 控制平面验收](../../consumers/axum-0.7.0-evidence.md)
- [0.7.1 文件服务验收](../../consumers/react-filesystem-0.7.1-evidence.md)
