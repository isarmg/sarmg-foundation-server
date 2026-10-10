# 选择配置

xcss 的产品接入由 `xcss-product.toml` 声明。产品的业务配置字段、环境变量和部署路径由产品定义；公共库提供解析、来源标记与校验机制。

## 运行形态

| Profile | 使用场景 | 主要必需能力 |
|---|---|---|
| `server-control-plane` | SQLite 持久管理面，Axum 与 React 管理 Web | platform-sqlite、admin-persistent、server-runtime、server-health |
| `server-filesystem` | 静态管理员与文件根，Axum | admin-static、memory-sessions、server-runtime、server-health、filesystem-root、linux-openat2 |
| `web-react-admin` | React 管理外壳和页面 | 由对应服务端组件选择 |
| `web-embedded-native` | 原生 ESM 管理页面 | 由对应服务端组件选择 |

完整组合、必需/可选能力和平台常量见 [profiles](../profiles/)；清单字段见 [JSON Schema](../schemas/xcss-product.schema.json)。带管理 Web 的服务端声明 `embedded-web` 并提供 `xcss-web-build.json`。

`foundation.version` 和 `foundation.git_rev` 与 Cargo/npm 输入保持一致。管理员会话时限、Argon2 参数、Cookie 和编译目标来自选定 profile 的公共策略，产品清单只选择支持的组合。

## 产品配置优先级

`xcss::config::resolve` 按“默认值 → JSON 文件 → 明确映射的环境变量 → 显式命令行覆盖”组合配置，后者优先。每层先按产品 Serde 类型校验；`resolve_validated` 可在各层检查字段的内在约束，最终配置再验证必需凭据和外部路径等运行前提。

`Loaded.sources` 记录每个有效叶字段的来源。文件与环境 JSON 各最多 1 MiB，覆盖项最多 256，数组整体替换。完整错误语义见[配置、CLI 与日志参考](configuration-cli-logging.md)。

## Web 工作区

`createXcssAdminApplication` 可省略 `workspace` 使用默认外观，也可提供部分选项：

| 选项 | 默认值 / 可选值 |
|---|---|
| `appearance` | `content-blocks`；可使用自己的小写外观标识 |
| `selection` | `underline`；也可 `custom` |
| `headerControls` | `icons`；也可 `text` |
| `fontFamily` | 内置 Maple 字体栈；可提供自有字体栈与资产 |
| `instanceNameMaxCharacters` | `32`；允许 `1`–`32` |
| `diagnostics` / `showVersion` | 固定 `false` |
| `navigationPlacement` / `headerIconSize` | 固定 `header` / `1em` |

详见[工作区配置](admin-workspace.md)。语言按 URL 参数、保存偏好、浏览器语言选择，支持 `zh-CN` 与 `en`；主题默认跟随系统，用户选择可保存。规则见[语言](admin-web-language.md)与[账号和主题](web-account-settings.md)。

React 构建的单资源预算默认 512 KiB，`maxAssetBytes` 接受 `1`–`67108864` 字节的安全整数；原生 ESM 默认 256 KiB。生产资源通过[内嵌构建](reference/embedded-web.md)与服务端一起交付。
