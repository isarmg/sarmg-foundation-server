# 02. 固定工具链、安装与第一次验证

## 2.1 为什么必须固定到补丁版本

xcss 的产物会进入多个产品。Rust、Node、TypeScript、React或Vite的一个补丁变化都可能影响编译、
类型、构建资源包、lock或软件包内容。这里不使用“我的版本更高所以应该兼容”的假设，而把工具链当成发布
输入。

| 工具 | 当前值 | 查验位置 |
|---|---:|---|
| Rust | `1.99.0` | `rust-toolchain.toml` |
| edition/MSRV | 2024 / `1.99` | 根 `Cargo.toml` |
| Node | `26.7.0` | `.node-version` |
| pnpm | `10.34.6` | 根 `package.json#packageManager` |
| TypeScript / Node types | `7.0.2` / `26.6.4` | 软件包清单 |
| React/DOM | `19.3.0` | `package.json` |
| Vite/plugin | `8.3.3` / `6.1.2` | `package.json` |

消费者Web使用npm并不冲突：xcss 内部 pnpm 管理单个服务端 Web 包；共享断言统一Node/React/Vite/TypeScript，
不要求产品改包管理器。

## 2.2 安装Rust

```bash
rustup toolchain install 1.99.0 --profile minimal --component rustfmt,clippy
rustup show active-toolchain
rustc --version
cargo --version
```

进入仓库后工具链文件会选中1.99.0。不要在命令中随意改用stable/nightly来通过检查。若某依赖只在更新
编译器上工作，应把工具链升级作为独立大问题，而不是在某台机器临时绕过。

## 2.3 安装Node与pnpm

用团队选择的版本管理器安装Node 26.7.0，然后验证：

```bash
node --version
pnpm --version
```

若pnpm缺失，安装精确10.34.6；安装方式取决于受信环境，但最终输出必须相同。随后：

```bash
pnpm install --frozen-lockfile --ignore-scripts
```

`--frozen-lockfile` 要求清单与`pnpm-lock.yaml`一致；`--ignore-scripts`避免依赖安装阶段运行不必要的
第三方脚本。真正需要的本仓构建与测试由显式命令执行。

## 2.4 在运行前先看工作树

```bash
git status --short
git diff --stat
```

当前任务可能同时修改多个仓库。`target`、`node_modules`、`dist`是生成物；源码、lock、Schema、测试夹具和
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
└─ isolated npm install + TypeScript + Vite

消费者层
├─ 本地path/file联调
├─ 不可变rev/tgz
├─ 独立checkout完整CI
└─ 编译后断网运行
```

上层通过不能替代下层。例如TypeScript源码能编译，并不能证明`package.json#exports`指向真实文件；
xcss 单测通过，也不能证明 xscs 仍严格验证 Sunshine 上游 TLS，或 xszs 发行的二进制内嵌了正确 Web 资源。

## 2.6 仓库校验规则

```bash
python3 scripts/check-xcss.py
```

它执行四类现有检查：依赖与单体边界、服务端依赖目标、Web 锁文件、版本与工具链。产品接入通过 `xcss-conformance`
检查其当前清单、来源、Schema、Web 和发行声明；实际行为由产品测试验收。常见失败：

| 失败 | 不要做 | 正确做法 |
|---|---|---|
| version differs | 只改报错文件 | 找出版本变更范围并同步所有事实源 |
| 未知 member/package | 直接扩allowlist | 先完成共享能力准入和公开边界评审 |
| 内部独立包或本地依赖 | 扩展允许列表 | 使用同一包内部模块；正式外部依赖固定来源与锁文件 |
| 服务端依赖目标不符 | 绕过平台门禁 | 所有外部依赖只声明在 Linux AMD64 GNU 目标下 |

## 2.7 工作流校验规则

```bash
python3 scripts/check-workflow-supply-chain.py
```

它拒绝浮动action/runner/Node、缺超时、源码检出凭据、过大权限、YAML anchor以及不正确发行
trigger。该检查不是通用YAML linter，而是本仓精确安全合同；修改它时要同时添加invalid 测试夹具，证明新
规则不会被结构技巧绕过。

## 2.8 Rust分层验证

```bash
cargo fmt --all -- --check
cargo check --locked --all-targets --all-features
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-features
RUSTDOCFLAGS="-Dwarnings" cargo doc --locked --all-features --no-deps
```

- `fmt`只检查格式；
- `check`覆盖当前编译平台的全部 Cargo target 类型，并同时启用全部 feature；不代表所有 CPU/OS 或每种 feature 组合；
- `clippy`把warning当错误；
- `test`执行认证、合同、Schema、SQLite等正反例；
- `doc`保证公开API链接和示例不会腐烂。

`xcss::server_target`在当前开发主机必须处于GNU/Linux AMD64，否则工作区会按设计编译失败。非目标
服务端的负向验证应由专门target gate测试完成，不能删除crate依赖。

## 2.9 Web与软件包验证

```bash
pnpm typecheck
pnpm test
python3 scripts/package-artifacts.py smoke
```

`pnpm test` 先构建根 dist，再从 dist 测试。软件包冒烟验证生成唯一的真实 tgz、审查 tar，并在临时空目录
用 npm 安装该包及精确外部 peer，执行 TypeScript 消费和 Vite 构建；依赖准备允许联网。真实消费者使用 npm，
因此最终安装验证不能只依赖本仓已有的 node_modules 或构建缓存。

## 2.10 Python工具测试

```bash
python3 -m unittest discover -s tools/tests -p 'test_*.py'
```

这些测试包含大量“恶意或损坏输入”：重复 JSON key、path 遍历、linked file、TOCTOU替换、文件
增长、错误mode/size/hash、超限树、错误workflow权限和软件包内容。安全工具的价值主要来自这些负例，
不能只跑一个正常发行目录。

## 2.11 统一执行顺序

开发时随每组代码和文档运行定向检查；最终按运维文档执行一次完整门禁。测试输出、临时DB、coverage和tgz不提交，除非某文件本来就是受审测试夹具。

## 2.12 本章练习

1. 找出Node版本的至少四个事实源，解释为何只改`.node-version`不够。
2. 说明`pnpm test`与软件包冒烟验证分别能发现什么。
3. 在产品的测试清单中把完整 Git 修订号改成分支名，解释现有来源检查为何拒绝。
4. 解释为何非AMD64 服务端编译失败是成功的负向测试，而不是“需要兼容”的bug。
5. 列出可安全删除的四类生成物和不可删除的四类事实源。
