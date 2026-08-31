# 02. 工具链、安装与第一次验证

## 2.1 固定环境

Rust 固定 `1.98.0`，Web 使用 pnpm `10.12.1` 和 lockfile 固定的 TypeScript。不要以全局最新工具直接
改写锁图。

```bash
rustup toolchain install 1.98.0
corepack enable
pnpm install --frozen-lockfile
```

## 2.2 仓库基线

```bash
cargo +1.98.0 metadata --no-deps
cargo +1.98.0 test --workspace
pnpm typecheck
pnpm build
pnpm test
```

Web tests 从已构建 `dist`/package exports 导入，以证明发布内容而非仅源码内部路径可用。

## 2.3 第一个 Rust 练习

在临时消费者中用 workspace path 依赖 `sarmg-error`，构造合法 envelope，再尝试大写、空 code、超长 code
和非对象 details。观察错误发生在构造边界，而不是序列化后。

## 2.4 第一个 SQLite 练习

在临时目录调用 `open_pool`，创建自己的表，检查 WAL、foreign_keys、synchronous 和 busy timeout。再并发
持有事务触发 busy，理解 helper 只返回准确错误，不为消费者设计 writer 架构。

## 2.5 第一个 Web 练习

从 `@sarmg/contracts` 导入 runtime guard，从 `@sarmg/http-client` 发起受控 JSON 请求，并引入 tokens CSS。
分别验证 TypeScript 编译、运行时坏 JSON 拒绝和 package export 路径。

## 2.6 常见环境错误

| 现象 | 原因 |
|---|---|
| frozen lock 失败 | package metadata 与 lock 不一致 |
| test 找不到 dist | 未先 build 或 exports/copy 配置错误 |
| Cargo 包名不存在 | workspace member/path/新名称未同步 |
| CSS token 缺失 | 导入了错误 export 或未构建 package |
| SQLite URL 失败 | 消费者路径/父目录未准备，而非 helper 应迁移 |

## 2.7 清理原则

不要提交 `target`、`node_modules`、临时消费者、真实产品数据或无意生成的 dist。锁文件属于源代码，依赖
变化必须显式评审。

## 2.8 完成标准

能从 package export 使用全部当前组件，能解释 build 与 test 的先后关系，并确认本地工具链未悄悄升级
依赖图。
