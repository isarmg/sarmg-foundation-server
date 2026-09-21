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
中文和日文继续使用内置 CJK 字体；无外部 CDN。消费者可用 `fontFamily` 设置自己的字体栈，
并自行提供相应资产。`appearance: "my-brand"` 可退出默认内容块主题，不要求消费者复刻该外观。
默认主题按钮单击切换亮/暗；默认选中项以文字下划线表示，不填充蓝底。
键盘 focus-visible 提示仍保留，它不是选中背景，不得为去除选中效果而删除焦点指示。

`InstanceHeaderActions` 将产品的新建、刷新动作放入右上角，随后是主题、退出。
`InstancePageNavigation` 提供统一的 `instances`、`details`、`logs` 三页状态和本地化标签。除 Dufs RAM 外，
所有 Server 管理页都使用这一导航：实例列表包含总览和每个实例摘要，详细信息合并状态与配置，日志集中展示业务记录。
Foundation 管理页面只提供全宽内容区域，不提供实例侧栏或详情页实例选择器。产品只提供页面内容和当前选中实例，
不再各自定义一套页面枚举或导航文案；用户切换实例时返回“实例列表”，选择目标后进入详细信息。
创建动作没有传入时不显示“+”，不虚构产品不支持的 API。
`InstanceNameField` 与 `validInstanceName` 按 Unicode 字符计数（与 Rust `chars()` 一致），
名称去除首尾空白后为 1–32 字符，禁止控制字符；消费者可配置更严格的上限，不能超过服务端上限。
后端仍必须校验请求，不能依赖浏览器输入限制。该规则不是文件名、用户名或路径长度规则。

原生 Web 通过 `native-workspace` 的 `configureNativeWorkspace` 使用同一配置和图标，
可用 `labels` 设置操作区、刷新、亮/暗模式和退出的可访问名称，保留消费者语言；未覆盖时退出沿用原按钮的 `aria-label`。
保留已有创建/退出事件处理，不引入 React 或第二套认证。

例如，英文消费者可传入 `labels: { actions: "Global actions", refresh: "Reload page",
light: "Switch to light mode", dark: "Switch to dark mode", logout: "Sign out" }`。
这些名称用于屏幕阅读器和悬停提示，不改变图标大小或业务回调。

`HeaderNavigation` 将产品页面选项放入顶部，与四个全局图标同一行；保留原有字体大小，
SVG 高度使用 `1em` 匹配文字，点击区域高 44px，窄屏仅导航区域横向滚动，不挤压右侧图标或隐藏键盘焦点。
`headerIconSize` 固定为 `1em`，不通过改变字号匹配图标。诊断和版本显示已移除，`diagnostics` / `showVersion`
只接受 `false`，不会保留可重新启用的旧面板、解析器或版本渲染代码。
产品版本仍用于协议校验及发布身份，不因删除显示而改变。
菜单栏左侧保留项目名称，只移除版本。名称和导航处于同一可滚动区域，窄屏不会挤出右侧操作图标。
`InstanceWorkspace`、`WorkspaceConfig.layout`、`emptyInstanceSidebar`、`showSidebar` 以及原生适配器的
`content`、`instanceName`、`instanceHref`、`labels.instances` 已从公共 API 删除。产品不得用私有侧栏、下拉框或
CSS 隐藏方式恢复第二套实例切换入口。

服务端平台路由不再注册 `/api/v2/platform/diagnostics`，匿名及已登录请求均返回 404；
登录、权限、Request ID、健康检查、内部任务监督及日志不受影响。
产品直接使用精确 Git revision 固定的 Foundation Runtime 路由入口，不再同步平台路由源码或调用旧诊断入口。

| 产品 | 实例列表对象 | 新建动作 |
| --- | --- | --- |
| Sunshine Manager | Client 管理实例 | 新建实例并生成配对码 |
| Host Monitoring | 已配对监控实例 | 新建实例并生成配对码 |
| Sentinel Monitor | 摄像头 | 新建摄像头 |
| Media Backup | 备份客户端实例 | 只填写名称并直接生成实例和配对码 |
| Dufs | 单个共享根目录 | 在当前目录新建文件夹；不支持远程多实例 |

## 当前不可变包分发

当前控制平面消费者升级到 Foundation 0.8.9；Dufs 的独立采用状态由消费者矩阵记录。
Rust 使用精确版本与完整 Git revision，Web 使用正式 Release tarball URL 和 lockfile integrity。
Shell、字体、主题及语言模块直接来自这些包，不执行旧快照同步脚本，不把平台源码复制到消费者。
历史独立构建及发行证据见 [0.7.0 记录](../consumers/axum-0.7.0-evidence.md)与 [0.7.1 记录](../consumers/react-filesystem-0.7.1-evidence.md)。

后续变更仍须发布新不可变版本、更新消费者锁图并复验，不覆盖旧制品；消费者只使用同一个 Shell Context。
通用原生入口是可选 Profile 的能力，不是 Dufs 当前实现。
