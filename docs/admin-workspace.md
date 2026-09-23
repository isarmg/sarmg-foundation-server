# 管理 Web 的默认工作区配置

公共默认配置位于 `packages/admin-shell/src/workspace-config.ts`，React 入口为
`createSarmgAdminApplication({ ..., workspace })`。消费者可省略 `workspace` 使用默认值，
也可以选择自己的 appearance、字体、选中样式和图标/文字操作区。
认证、CSRF、管理员权限及产品任务逻辑均不受外观选择影响。

```tsx
createSarmgAdminApplication({
  product, client, navigation: [], routes: <ProductWorkspace />,
  workspace: {
    appearance: "content-blocks",
    selection: "underline",
    headerControls: "icons",
    instanceNameMaxCharacters: 32,
    diagnostics: false,
    showVersion: false,
    navigationPlacement: "header",
    headerIconSize: "1em",
  },
});
```

默认英文为 Maple Mono **Normal NL Regular/Bold**：正体、非手写字形、无连字。
中文和日文使用内置 CJK 字体；无外部 CDN。消费者可用 `fontFamily` 设置自己的字体栈，
并自行提供相应资产。`appearance: "my-brand"` 可退出默认内容块主题，不要求消费者复刻该外观。
默认主题按钮单击切换亮/暗；默认选中项以文字下划线表示，不填充蓝底。
键盘 focus-visible 提示仍保留，它不是选中背景，不得为去除选中效果而删除焦点指示。

`InstanceHeaderActions` 将消费者传入的新建、刷新动作放入右上角，随后是语言、主题、退出和账号设置。
`InstancePageNavigation` 提供 `instances`、`details`、`logs` 三页状态和本地化标签，
通过 `navigate` 回调通知产品切换页面，`detailsDisabled` 控制详细信息入口是否可用。
Foundation 管理页面提供全宽内容区域。页面内容、选中实例、导航状态及对象的业务含义均由产品负责。
创建动作没有传入时不显示“+”，不虚构产品不支持的 API。
`InstanceNameField` 与 `validInstanceName` 按 Unicode 字符计数（与 Rust `chars()` 一致），
名称去除首尾空白后为 1–32 字符，禁止 ASCII/C1 控制字符和孤立代理项；消费者可配置 1–32 的整数上限。
初始值、受控值更新和用户输入使用同一校验，长度按去除首尾空白后的名称计算。
后端仍必须校验请求，不能依赖浏览器输入限制。该规则不是文件名、用户名或路径长度规则。

原生 Web 通过 `native-workspace` 的 `configureNativeWorkspace` 使用同一配置和图标，
可用 `labels` 设置操作区、刷新、亮/暗模式和退出的可访问名称，保留消费者语言；未覆盖时退出沿用原按钮的 `aria-label`。
保留已有创建/退出事件处理，不引入 React 或第二套认证。

例如，英文消费者可传入 `labels: { actions: "Global actions", refresh: "Reload page",
light: "Switch to light mode", dark: "Switch to dark mode", logout: "Sign out" }`。
这些名称用于屏幕阅读器和悬停提示，不改变图标大小或业务回调。

`HeaderNavigation` 将产品页面选项放入顶部，与全局操作同一行；使用正文的字体大小，
SVG 高度使用 `1em` 匹配文字，点击区域高 44px，窄屏仅导航区域横向滚动，不挤压右侧图标或隐藏键盘焦点。
`headerIconSize` 固定为 `1em`，`diagnostics` / `showVersion` 只接受 `false`。
产品版本用于协议校验及发布身份。菜单栏左侧显示项目名称，名称和导航处于同一可滚动区域，
窄屏不会挤出右侧操作图标。

服务端平台路由对 `/api/v2/platform/diagnostics` 的匿名及已登录请求均返回 404；
登录、权限、Request ID、健康检查、内部任务监督及日志不受影响。
产品直接使用精确 Git revision 固定的 Foundation Runtime 路由入口。
实例创建、配对、文件操作等业务行为由消费者回调和协议定义，Foundation 不按产品名称分支。

## 当前不可变包分发

当前源码版本见根 README；各产品采用的版本与验收状态由消费者矩阵记录。
Rust 使用精确版本与完整 Git revision，Web 使用正式 Release tarball URL 和 lockfile integrity。
Shell、字体、主题及语言模块直接来自这些包；独立构建不需要同级 Foundation 源码。
历史独立构建及发行证据见 [0.7.0 记录](../consumers/axum-0.7.0-evidence.md)与 [0.7.1 记录](../consumers/react-filesystem-0.7.1-evidence.md)。

后续变更仍须发布新不可变版本、更新消费者锁图并复验，不覆盖旧制品；消费者只使用同一个 Shell Context。
通用原生入口是 `web-embedded-native` Profile 的能力；消费者选择的 Profile 由其清单声明。
