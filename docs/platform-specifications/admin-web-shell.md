# 当前管理员 Web Profile

产品通过 `createSarmgAdminApplication({ product, navigation, routes, client? })` 提供产品身份、同站导航和业务内容。
可传入唯一共享管理员 client；未传时由 Shell 创建。业务通过 `useAdminApplication()` 取得 client、已认证
Session 和有界 `notify`。认证数据仅保存在 admin-web 内存中，不放入浏览器存储。

Shell 拥有登录、Session 恢复/重试、退出、导航、跳过导航链接、主题切换、错误边界和通知，不提供诊断入口或面板。
登录期间表单保持挂载，防止失败提示丢失；密码失败后清空并恢复焦点。对外仅显示固定安全提示及校验后的 Request ID，
不显示 error.message、stack 或任意内部状态 JSON。管理 Web 不主动请求诊断接口；服务运行所需的健康检查不受影响。

`@sarmg/admin-ui` 提供当前 UI primitives。Dialog 使用 native modal，并明确封闭键盘 Tab 循环、支持 Escape
和恢复原焦点。危险确认默认聚焦取消。Button 默认 type=button，IconButton 强制可访问名称。
通知最多五条，可由键盘关闭，位于页头下方正常文档流中，不叠加遮挡业务按钮或键盘焦点。
Table 可键盘横向滚动；loading/error 有可见文本与正确 live/alert 语义。

React 产品统一导入 design-tokens 的 tokens.css、tokens.dark.css、reset.css、accessibility.css，
web-fonts/fonts.css 及 admin-ui/styles.css。不再维护产品私有登录页、全局 Shell 或字体副本。
主题初始采用系统亮暗偏好，顶部图标按钮每点击一次切换浅色/深色；按钮提供可访问名称、键盘操作和说明。
遵守 forced-colors 和 reduced-motion；业务布局仍由产品负责。空 `navigation` 可用于产品自己的实例分栏，不输出多余导航栏。

默认视觉外观采用 Union 内容块：登录卡片宽度上限 380px、3:2、六行布局；
业务摘要使用比例卡片，长表单、详情、表格使用可伸展面板，不能裁掉功能或错误信息。
`@sarmg/admin-ui/styles.css` 自动加载此外观，不要求消费者另行选择。
消费者允许自行设计其他外观：在 `html` 设置 `data-sarmg-appearance="custom"`
（或任意非 `content-blocks` 的明确名称）退出默认外观，再加载自己的样式。
视觉替换不授权复制认证实现、改变权限合同或降低无障碍验收。
默认字体独立于外观管理，切换外观不自动替换字体。
详见 [外观接入及源码快照说明](../../packages/admin-ui/CONTENT-BLOCKS.md)。

`@sarmg/web-toolchain/vite` 和 `@sarmg/web-toolchain/tsconfig.json` 是唯一工具链入口，
admin-web 不再导出构建配置。构建关闭 source maps，对每个产物执行硬性大小预算（默认 512 KiB），超限失败。
输出目录为 dist，资源由 Vite 生成内容哈希文件名，React/React DOM 去重。

验收命令：`pnpm test`、`pnpm test:web`。浏览器套件在 Chromium 和 Firefox 检查登录/退出、Request ID、
焦点循环、错误边界、诊断入口移除、主题和通知，并对 360px 移动宽度的两种主题运行 axe WCAG AA 与横向溢出检查。
测试只代表共享 Profile；每个消费者仍须运行其业务和独立构建验收，不能据此宣称全产品改造完成。
