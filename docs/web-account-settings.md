# 当前账号设置页

点击菜单栏右侧的 `AccountSettings` 人物图标进入 `#account`。图标没有选中状态、形状变化或下划线。产品在自己的菜单路由中渲染 `AccountPage`，保留顶部导航与全局操作；账号内容使用普通的 `xcss-content-panel`，支持深浅色主题。

页面提供用户名（默认当前用户名）、当前密码、新密码、确认新密码和保存按钮。当前密码由原有服务端接口验证后原子更新账号；错误密码不产生修改，新密码留空保留原密码。新密码长度和两次输入在前端校验，服务端继续执行完整验证。请求失败后清除密码，不显示请求标识或后端内部错误。保存成功后现有会话失效，需要重新登录；离开页面会丢弃未保存的密码。

在标准 Shell 的产品路由中直接使用：

```tsx
import { AccountPage } from "@xcss/admin-shell";
// 路由状态与其他菜单一样响应 hashchange。
return page === "account" ? <AccountPage /> : <ProductPage />;
```

自定义 Shell 通过 `<AccountPage client={client} username={session.username} onUpdated={returnToLogin} />` 接入同一表单。`AccountSettings` 的 `onNavigate` 可对接产品自己的导航；默认进入 `#account`。实例菜单的 `InstancePageNavigation` 接受 `"account"` 页面状态，此时三个业务菜单均不显示选中标记。

组件、样式和主题直接来自正式 Foundation npm 包；消费者固定发布版本、tarball 和 integrity，不使用源码快照或 Vite 替换插件。登录页保留 3:2 灰色内容块，浅色模式下字体为黑色；语言和主题图标使用与管理页相同的 `1em` 尺寸。主题默认跟随系统，手动选择保存在 `sarmg:theme`，刷新、登录和退出后继续使用；存储不可用时当前文档内的选择仍有效。原 Host 审核版的主题选择可迁移。认证凭据始终不进入浏览器存储。
