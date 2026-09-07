# 默认内容块外观与消费者自定义

来源：union-rust 的 Apache-2.0 `sarmg-design` 内容卡片、登录卡片及色板。
默认 `@sarmg/admin-ui/styles.css` 自动导入此文件。
文档根没有 `data-sarmg-appearance` 属性或其值为 `content-blocks` 时启用。
消费者可以设置 `<html data-sarmg-appearance="custom">`（或其他自定义名称）退出，
再在基础样式之后加载自己的 CSS；所有内容块规则及色板都会退出作用域。
不替换认证组件，不影响 API、会话、权限或默认字体。单独的
`@sarmg/admin-ui/content-blocks.css` 导出也可用于原生 ESM 页面。

登录复用当前 AdminShell 的语义表单，呈现 380px、3:2、六行卡片、透明输入和文字操作。
标题与产品标识仍向辅助技术提供；错误行可以滚动，完整 request ID 不会被删除。
跟随现有 `data-theme="light|dark"` 和系统主题；保留强制色彩和键盘焦点。

业务可以使用 `.sarmg-content-grid`、`.sarmg-content-card`、
`.sarmg-content-card__inner`、`.sarmg-content-row` 构成六行 3:2 卡片。
长表单、详情、表格使用可伸展的 `.sarmg-content-panel`，不要强制塞进固定比例卡片。
这些类仅规定展示，不包含客户端或服务器行为。

## 当前发行接入

五个管理 Web 均使用正式 Foundation 包：四个控制平面消费者固定 0.7.0，Dufs 固定 0.7.1。
直接导入 `@sarmg/admin-ui/styles.css`；包已包含默认内容块 CSS，不再复制到产品 `clients/web/appearance/`，
也不执行旧的源码快照同步命令。消费者固定 Release tarball URL 与 lockfile integrity，独立构建不需要同级 Foundation 源码。

默认外观可由消费者自定义，不是对外部产品的强制品牌规则。字体由 `@sarmg/web-fonts` 提供，不复制旧版字体规则。
后续变更必须发布新不可变包并更新消费者锁文件，不覆盖已有 tarball。
实际版本和验收见 [0.7.0 记录](../../consumers/axum-0.7.0-evidence.md)与 [0.7.1 记录](../../consumers/react-filesystem-0.7.1-evidence.md)。
