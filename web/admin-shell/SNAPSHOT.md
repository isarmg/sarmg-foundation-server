# 管理 Shell 接入与验收

共享 Shell 提供亮/暗图标切换、顶部导航和全宽内容。认证客户端、CSRF 和管理员合同由正式 xcss 包提供。
公共默认值与消费者覆盖方式见 [工作区配置](../../docs/admin-workspace.md)。

产品从不可变 Release tarball 安装单个 `@xcss/web` 包并固定 lockfile integrity，再按 `admin-shell`、`admin-ui` 等公开子路径导入，
使用一套 Shell Context；独立构建不需要同级 xcss checkout。
通用原生 Web 入口供选择 `web-embedded-native` 的消费者使用。

各产品实际采用的版本与完整 revision 以其 manifest 和锁文件为准；验收结果对应产品的源码 commit、CI 和正式 Release。
