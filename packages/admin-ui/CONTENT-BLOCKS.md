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

当前五个管理 Web 显式启用。尚未发布的外观以受审阅的源码快照存放于各产品
`clients/web/appearance/`，构建验证 SHA-256。同步命令：

```sh
node scripts/sync-content-blocks.mjs /absolute/product-repository
```

引入产品快照的 CSS，置于基础和业务 CSS 之后；在构建检查中导入快照 `verify.mjs`。
快照验证器检查这五个第一方产品已启用默认外观；这不是对外部消费者的强制规则。
已发布的 Foundation 0.6.0 包、锁文件、认证依赖保持不可变；新 CSS 导出随未来新版本发布，
不得覆盖 0.6.0 的 tarball。字体仍由 web-fonts 独立管理，不复制旧版字体规则。
