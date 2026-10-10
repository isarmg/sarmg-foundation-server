# 构建与测试

开发环境为 Linux x86_64 GNU、Rust `1.99.0`、Node `26.7.0`、pnpm `10.34.6`、Python 3 和 C 编译工具链。工具版本来自 `rust-toolchain.toml`、`.node-version` 和 `package.json`。

## 准备和构建

在仓库根目录执行：

```sh
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy
node --version
pnpm --version
pnpm install --frozen-lockfile --ignore-scripts
cargo build --release --locked
pnpm build
```

Rust 产物位于 `target/release/`，Web 公开输出位于根 `dist/`。构建没有启动独立 xcss 服务。

## 运行检查

```sh
python3 scripts/check-xcss.py
python3 scripts/check-rust-package-licenses.py
python3 scripts/check-workflow-supply-chain.py
python3 -m unittest discover -s tools/tests -p 'test_*.py'
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --all-features --no-deps
pnpm typecheck
pnpm test
pnpm test:web
python3 scripts/package-artifacts.py smoke
git diff --check
```

`pnpm test` 从构建后的 dist 执行测试；`test:web` 使用 Playwright 的 Chromium/Firefox，需要已安装对应浏览器及系统依赖。`smoke` 生成真实 tgz，在独立目录安装精确 peers，验证导出、TypeScript 和 Vite，准备依赖时可联网。

## 生成 Web 归档

使用新的或空的输出目录：

```sh
python3 scripts/package-artifacts.py release --output release/npm
```

产物为 `release/npm/xcss-web-1.0.0.tgz`，可用于本地产品接入验证。正式资产的源码绑定、标签和回下载检查见[发行工具操作](operations.md)。

## 验证实际消费者

在 xcss 根目录执行，将路径换成产品的绝对路径：

```sh
python3 scripts/xcss-conformance.py verify-source --product-root /absolute/product
python3 scripts/xcss-conformance.py verify-web --product-root /absolute/product
python3 scripts/xcss-conformance.py report --product-root /absolute/product --json
python3 scripts/xcss-conformance.py verify-release --product-root /absolute/product --require-published
```

源码报告标明清单、依赖、Schema、Web 和发行状态。发行验证需要真实发布清单；随后在产品仓库构建、测试实际路由与业务，并验证其最终制品。

## 修改公共能力

按[改动工作流](project-workflow.md)同步 API、实现、测试和消费者。认证、合同、数据库或文件改动的专项场景见[测试参考](reference/testing.md)，原生调用见[unsafe 审查](unsafe-audit.md)。
