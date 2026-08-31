# 02. 固定工具链、安装与第一次验证

## 2.1 为什么必须固定到 patch 版本

Foundation 的产物会进入多个产品。Rust、Node、TypeScript、React或Vite的一个patch变化都可能影响编译、
类型、bundle、lock或package内容。这里不使用“我的版本更高所以应该兼容”的假设，而把工具链当成发布
输入。

| 工具 | 当前值 | 查验位置 |
|---|---:|---|
| Rust | `1.98.0` | `rust-toolchain.toml` |
| edition/MSRV | 2024 / `1.98` | 根 `Cargo.toml` |
| Node | `26.7.0` | `.node-version` |
| pnpm | `10.12.1` | 根 `package.json#packageManager` |
| TypeScript | `5.8.3` | package manifest |
| React/DOM | `19.2.8` | `packages/admin-web/package.json` |
| Vite/plugin | `7.3.6` / `4.7.0` | `packages/admin-web/package.json` |

消费者Web使用npm并不冲突：Foundation内部pnpm只管理monorepo；共享断言统一Node/React/Vite/TypeScript，
不要求产品改包管理器。

## 2.2 安装Rust

```bash
rustup toolchain install 1.98.0 --profile minimal --component rustfmt,clippy
rustup show active-toolchain
rustc --version
cargo --version
```

进入仓库后toolchain文件会选中1.98.0。不要在命令中随意改用stable/nightly来通过检查。若某依赖只在更新
编译器上工作，应把工具链升级作为独立大问题，而不是在某台机器临时绕过。

## 2.3 安装Node与pnpm

用团队选择的版本管理器安装Node 26.7.0，然后验证：

```bash
node --version
pnpm --version
```

若pnpm缺失，安装精确10.12.1；安装方式取决于受信环境，但最终输出必须相同。随后：

```bash
pnpm install --frozen-lockfile --ignore-scripts
```

`--frozen-lockfile` 要求manifest与`pnpm-lock.yaml`一致；`--ignore-scripts`避免依赖安装阶段运行不必要的
第三方脚本。真正需要的本仓build/test由显式命令执行。

## 2.4 在运行前先看工作树

```bash
git status --short
git diff --stat
```

当前任务可能同时修改多个仓库。`target`、`node_modules`、`dist`是生成物；源码、lock、Schema、fixture和
文档是事实源。不要清理或覆盖你不理解的已有修改，也不要把另一个人的改动误当测试产物删除。

## 2.5 先理解验证层次

```text
静态policy
├─ repository identity/version/member/toolchain
└─ workflow supply-chain结构

语言层
├─ Rust fmt/check/clippy/test/doc
└─ TypeScript typecheck/package tests

发布形态
├─ clean dist
├─ real tgz
├─ tar member检查
└─ empty-directory offline install

消费者层
├─ 本地path/file联调
├─ 不可变rev/tgz
├─ 独立checkout完整CI
└─ 编译后断网运行
```

上层通过不能替代下层。例如TypeScript源码能编译，并不能证明`package.json#exports`指向真实文件；
Foundation单测通过，也不能证明Sunshine仍强制上游TLS或Media release仍包含正确Web dist。

## 2.6 Repository policy

```bash
python3 scripts/check-foundation.py
```

它会检查版本、6个crate、4个package、精确内部依赖、工具链、consumer matrix和取消名称。常见失败：

| 失败 | 不要做 | 正确做法 |
|---|---|---|
| version differs | 只改报错文件 | 找出版本变更范围并同步所有事实源 |
| unknown member/package | 直接扩allowlist | 先完成共享能力准入和公开边界评审 |
| internal dep not exact | 改成caret | 使用精确当前版本并更新lock |
| matrix inconsistent | 手改passing | 完成真实不可变依赖验证后填证据 |
| contains cancelled name | 绕过字符串扫描 | 修改真实身份、import、文档和资产名 |

## 2.7 Workflow policy

```bash
python3 scripts/check-workflow-supply-chain.py
```

它拒绝浮动action/runner/Node、缺timeout、checkout credential、过大权限、YAML anchor以及不正确release
trigger。该检查不是通用YAML linter，而是本仓精确安全合同；修改它时要同时添加invalid fixture，证明新
规则不会被结构技巧绕过。

## 2.8 Rust分层验证

```bash
cargo fmt --all -- --check
cargo check --locked --workspace --all-targets --all-features
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --workspace --all-features --no-deps
```

- `fmt`只检查格式；
- `check`覆盖所有target类型和feature组合的编译；
- `clippy`把warning当错误；
- `test`执行认证、合同、Schema、SQLite等正反例；
- `doc`保证公开API链接和示例不会腐烂。

`sarmg-server-target`在当前开发主机必须处于GNU/Linux AMD64，否则workspace会按设计compile-fail。非目标
Server的负向验证应由专门target gate测试完成，不能删除crate依赖。

## 2.9 Web与package验证

```bash
pnpm typecheck
pnpm test
python3 scripts/package-artifacts.py smoke
```

`pnpm test`会先构建需要的dist并从dist测试。package smoke还会生成4个tgz、审查tar，再用npm在临时空目录
离线安装全部包。为什么使用npm做最终安装？因为真实消费者使用npm，这能发现pnpm workspace未暴露的问题。

## 2.10 Python工具测试

```bash
python3 -m unittest discover -s tools/tests -p 'test_*.py'
```

这些测试包含大量“恶意或损坏输入”：duplicate JSON key、path traversal、linked file、TOCTOU替换、文件
增长、错误mode/size/hash、超限树、错误workflow权限和package内容。安全工具的价值主要来自这些负例，
不能只跑一个正常release tree。

## 2.11 统一执行顺序

当前项目约定是先完成代码和文档，再一起运行、反馈、修复。统一阶段按运维文档完整顺序跑；任何修复后
先跑定向层，最终再跑全套。测试输出、临时DB、coverage和tgz不提交，除非某文件本来就是受审fixture。

## 2.12 本章练习

1. 找出Node版本的至少四个事实源，解释为何只改`.node-version`不够。
2. 说明`pnpm test`与package smoke分别能发现什么。
3. 构造一个虚假的consumer matrix passing条目，预测policy会因哪些字段拒绝。
4. 解释为何非AMD64 Server compile-fail是成功的负向测试，而不是“需要兼容”的bug。
5. 列出可安全删除的四类生成物和不可删除的四类事实源。
