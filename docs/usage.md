# 组合公共模块

按产品需要使用下列入口。完整 Rust API 可用 `cargo doc --locked --all-features --no-deps --open` 阅读，Web 入口由根 `package.json` 的 exports 定义。

## 服务端接入顺序

1. 用 `xcss::config` 组合产品配置，再验证实际运行前提。
2. 用 `fs_safety` 和 `state_file` 打开产品私有状态、取得运行锁。
3. 选择 `admin_sqlite` 或 `admin_static`，验证当前管理员状态。
4. 挂载公共认证适配器，再组合产品业务路由。
5. 向 `ServerRuntime` 注册健康检查、后台任务和最后关闭资源的参与者，运行 HTTP 服务。

具体初始化和配置命令由产品提供。文件权限、诊断连接与锁的详细前提见[文件 API](filesystem-handles.md)、[配置和状态](configuration-cli-logging.md)、[运行时](platform-specifications/server-runtime.md)。

## 管理 Web

`createXcssAdminApplication` 组合产品身份、navigation、routes 和可选的共享 client。Shell 负责登录、恢复会话、退出、主题和通知；产品的业务组件通过 `useAdminApplication()` 获取 client、session 和 notify。

React 产品加载以下共享样式：

```css
@import "@xcss/web/design-tokens/tokens.css";
@import "@xcss/web/design-tokens/tokens.dark.css";
@import "@xcss/web/design-tokens/reset.css";
@import "@xcss/web/design-tokens/accessibility.css";
@import "@xcss/web/web-fonts/fonts.css";
@import "@xcss/web/admin-ui/styles.css";
```

业务响应使用与产品 DTO 对应的运行时 guard。管理员会话和 CSRF 留在内存，刷新页面通过服务端 HttpOnly Cookie 恢复。修改请求超时可能已在服务端完成，按业务操作 ID 查询结果后决定是否重试。

布局与组合见[Shell](platform-specifications/admin-web-shell.md)；账号页见[账户设置](web-account-settings.md)；日期筛选见[日期控件](web-date-range.md)；请求预算和认证竞态见[HTTP 参考](reference/http-client.md)。

## 日志与诊断

通过 `LogRecord` 记录稳定事件与实例、请求或任务 ID，产品字段保持在自己的命名空间。默认写 stderr；安装轮转 sink 后，普通 emit 与 tracing layer 共享文件输出。默认单文件 8 MiB，保留 4 个归档，总计 40 MiB。

使用 `rejected_count` 观察 tracing 输出失败。日志与错误保留脱敏字段，详细查询条件和上限见[日志参考](configuration-cli-logging.md#通用结构化日志)。

## 构建和交付

管理 Web 通过 `xcss::web_assets` 内嵌到产品二进制。使用共同构建入口可保证先构建 Web，再编译 Rust，并核对实际资源；[内嵌构建参考](reference/embedded-web.md)给出配置和开发热更新步骤。随后执行产品自己的打包、安装与业务验收。
