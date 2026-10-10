# 开始接入 xcss

本页面向把 xcss 加入服务端产品的开发者。准备 Linux x86_64 GNU、Rust `1.99.0`；需要管理 Web 时再准备 Node `26.7.0` 与 npm。

## 1. 添加 Rust 依赖

将[根 README 的固定依赖](../README.md#快速部署)加入产品 `Cargo.toml`。在产品根目录运行 `cargo check` 生成或更新 `Cargo.lock`，并提交锁文件。下列示例使用已发布的 1.0.1。版本 `=1.0.1` 与完整 Git revision 共同确定库的构建输入。

在产品中创建 `examples/xcss_identity.rs`，加入以下示例：

```rust
use xcss::admin_auth::normalize_administrator_username;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let username = normalize_administrator_username(" Admin.Ops ")?;
    assert_eq!(username, "admin.ops");
    println!("{username}");
    Ok(())
}
```

在产品目录执行 `cargo run --locked --example xcss_identity`，预期输出 `admin.ops`。该示例验证依赖与一个公开 API；完整服务由产品选择认证存储、路由和生命周期适配。

## 2. 声明产品形态

产品根目录的 `xcss-product.toml` 说明使用的版本、revision、profile 和 capabilities。下面是持久管理员 + Axum + React 管理 Web 的示例；产品名称改为实际名称，revision 与 Cargo 依赖保持一致。

```toml
format = 1
product_id = "example-server"

[foundation]
platform_generation = 1
version = "1.0.1"
git_rev = "fd90ca39b8f03359a0ba92e182f7d84bc7c1a315"

[[components]]
id = "server"
profile = "server-control-plane"
http_adapter = "axum"
web_profile = "web-react-admin"
capabilities = ["embedded-web", "platform-sqlite", "admin-persistent", "server-runtime", "server-health"]
```

静态管理员和文件服务使用不同组合，见[选择配置](configuration.md)。声明 `embedded-web` 后还需按[内嵌 Web](reference/embedded-web.md)提供实际构建输入。

## 3. 接入管理 Web

在产品 Web 目录安装同版发行包和需要的精确 peers：

```sh
npm pkg set 'engines.node=>=26.7.0 <27'
npm install --save-exact https://github.com/isarmg/xcss/releases/download/v1.0.1/xcss-web-1.0.1.tgz
npm install --save-exact react@19.3.0 react-dom@19.3.0
npm install --save-dev --save-exact vite@8.3.3 @vitejs/plugin-react@6.1.2 typescript@7.0.2   @types/react@19.3.0 @types/react-dom@19.3.0 @types/node@26.6.4
```

提交生成的 `package-lock.json`；产品来源检查读取该锁文件的发行 URL 和完整性摘要。xcss 仓库自身用 pnpm 构建，产品消费路径使用 npm。`--save-exact` 保持清单中的精确版本，避免默认 `^` 范围与工具链检查冲突。

从公开子路径导入所需模块，例如：

```ts
import { createAdministratorApiClient } from "@xcss/web/admin-web";

export const administratorApi = createAdministratorApiClient();
```

浏览器默认使用当前来源；在一个应用中共享这个 client。Shell、页面与样式的组合见[使用公共模块](usage.md)。

## 4. 检查和构建产品

在 xcss 源码根目录运行，替换产品绝对路径：

```sh
python3 scripts/xcss-conformance.py report   --product-root /absolute/path/to/product --json
```

根据报告完成来源、Schema、Web 和发行声明，再构建并运行产品自身的测试。`report` 的 `release_verified` 只有实际发行清单通过验证才为 true；发行校验使用[正式验收入口](development.md#验证实际消费者)。
