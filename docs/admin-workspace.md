# 管理 Web 的默认工作区配置

公共默认配置位于 `packages/admin-shell/src/workspace-config.ts`，React 入口为
`createSarmgAdminApplication({ ..., workspace })`。消费者可省略 `workspace` 使用默认值，
也可以选择自己的 appearance、字体、布局、选中样式和图标/文字操作区。
认证、CSRF、管理员权限及产品任务逻辑均不受外观选择影响。

```tsx
createSarmgAdminApplication({
  product, client, navigation: [], routes: <ProductWorkspace />,
  workspace: {
    appearance: "content-blocks",
    layout: "instances",
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
中文和日文继续使用内置 CJK 字体；无外部 CDN。消费者可用 `fontFamily` 设置自己的字体栈，
并自行提供相应资产。`appearance: "my-brand"` 可退出默认内容块主题，不要求消费者复刻该外观。
默认主题按钮单击切换亮/暗；默认选中项以文字下划线表示，不填充蓝底。
键盘 focus-visible 提示仍保留，它不是选中背景，不得为去除选中效果而删除焦点指示。

`InstanceWorkspace` 左侧只显示实例名称，右侧由产品传入内容与设置；窄屏纵向排列。
`InstanceHeaderActions` 将产品的新建、刷新动作放入右上角，随后是主题、退出。
创建动作没有传入时不显示“+”，不虚构产品不支持的 API。
`InstanceNameField` 与 `validInstanceName` 按 Unicode 字符计数（与 Rust `chars()` 一致），
名称去除首尾空白后为 1–32 字符，禁止控制字符；消费者可配置更严格的上限，不能超过服务端上限。
后端仍必须校验请求，不能依赖浏览器输入限制。该规则不是文件名、用户名或路径长度规则。

原生 Web 通过 `native-workspace` 的 `configureNativeWorkspace` 使用同一配置和图标，
可用 `labels` 设置操作区、刷新、亮/暗模式、退出及实例区的可访问名称，保留消费者语言；未覆盖时退出沿用原按钮的 `aria-label`。
保留已有创建/退出事件处理，不引入 React 或第二套认证。

例如，英文消费者可传入 `labels: { actions: "Global actions", refresh: "Reload page",
light: "Switch to light mode", dark: "Switch to dark mode", logout: "Sign out",
instances: "Shared root instance" }`。这些名称用于屏幕阅读器和悬停提示，不改变图标大小或业务回调。

`HeaderNavigation` 将产品页面选项放入顶部，与四个全局图标同一行；保留原有字体大小，
SVG 高度使用 `1em` 匹配文字，点击区域高 44px，窄屏仅导航区域横向滚动，不挤压右侧图标或隐藏键盘焦点。
`headerIconSize` 固定为 `1em`，不通过改变字号匹配图标。诊断和版本显示已移除，`diagnostics` / `showVersion`
只接受 `false`，不会保留可重新启用的旧面板、解析器或版本渲染代码。
产品版本仍用于协议校验及发布身份，不因删除显示而改变。
菜单栏左侧保留项目名称，只移除版本。名称和导航处于同一可滚动区域，窄屏不会挤出右侧操作图标。
`emptyInstanceSidebar: "collapse"` 为默认规则：空实例列表不渲染侧栏、不预留宽度。
产品可在全局总览、系统管理等与实例选择无关的页面传入 `showSidebar={false}`，有实例时也使用全宽内容。
Sunshine 与 Host Monitoring 的顶部“实例”进入独立列表页；选中实例后进入业务内容，
不再在业务页旁常驻实例栏。其他产品保留各自实例栏。

服务端平台路由不再注册 `/api/v2/platform/diagnostics`，匿名及已登录请求均返回 404；
登录、权限、Request ID、健康检查、内部任务监督及日志不受影响。
产品直接使用精确 Git revision 固定的 Foundation Runtime 路由入口，不再同步平台路由源码或调用旧诊断入口。

| 产品 | 实例列表对象 | 新建动作 |
| --- | --- | --- |
| Sunshine Manager | Client 管理实例 | 新建实例并生成配对码 |
| Host Monitoring | 已配对监控实例 | 新建实例并生成配对码 |
| Sentinel Monitor | 摄像头 | 新建摄像头 |
| Media Backup | 备份用户 | 新建备份用户（不等同平台管理员） |
| Dufs | 单个共享根目录 | 在当前目录新建文件夹；不支持远程多实例 |

## 当前不可变包分发

四个控制平面消费者固定 Foundation 0.7.0，Dufs 固定 0.7.1 并使用 React Profile。
Rust 使用精确版本与完整 Git revision，Web 使用正式 Release tarball URL 和 lockfile integrity。
Shell、字体、主题及语言模块直接来自这些包，不执行旧快照同步脚本，不把平台源码复制到消费者。
独立构建及发行证据见 [0.7.0 记录](../consumers/axum-0.7.0-evidence.md)与 [0.7.1 记录](../consumers/react-filesystem-0.7.1-evidence.md)。

后续变更仍须发布新不可变版本、更新消费者锁图并复验，不覆盖旧制品；消费者只使用同一个 Shell Context。
通用原生入口是可选 Profile 的能力，不是 Dufs 当前实现。
