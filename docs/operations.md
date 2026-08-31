# Sarmg Foundation 运维文档

Foundation 没有生产 daemon，本运维文档面向仓库维护、依赖、构建、制品发布和消费者响应。

## 1. 固定工具链

- Rust `1.98.0`，Cargo workspace edition 2024。
- pnpm `10.12.1`，以 `pnpm-lock.yaml` 为唯一锁图。
- TypeScript `5.8.3` 由各 package devDependency 固定。

```bash
rustup toolchain install 1.98.0
corepack enable
pnpm install --frozen-lockfile
```

不要用无锁更新的本地依赖结果发布。

## 2. 完整质量门

```bash
python3 scripts/check-workflow-supply-chain.py
cargo +1.98.0 fmt --all -- --check
cargo +1.98.0 check --workspace --all-targets
cargo +1.98.0 clippy --workspace --all-targets -- -D warnings
cargo +1.98.0 test --workspace
pnpm typecheck
pnpm build
pnpm test
git diff --check
```

检查 `packages/*/dist` 由构建产生且 package exports 都能从构建输出导入。不要提交临时 `dist`、
node_modules 或 target，除非仓库以后明确改变发行政策。

## 3. 版本与发布原则

所有 package/crate 当前统一 `0.2.0`。发布前：

1. 列出所有 breaking changes 和受影响消费者。
2. 删除旧 API，不保留 alias/re-export。
3. 更新 Cargo/npm 版本与 lockfile。
4. 在 Foundation 跑完整门禁。
5. 在每个真实消费者 pin 精确版本、修改导入并跑该产品完整 CI。
6. 构建/检查 package 内容，只包含 `dist` 和必要元数据。
7. 发布不可变版本；禁止覆盖同版本资产。

## 4. 消费运行故障

| 现象 | 排查 |
|---|---|
| TS 找不到 export | 检查精确 package 版本、`exports` 和是否已 build |
| ErrorEnvelope guard 拒绝 | 对比 exact keys、ErrorCode、request_id/details 类型，禁止宽松 fallback |
| HTTP response too large | 调整业务分页；仅在评估内存后提高上限，不能超过 64 MiB |
| request_timeout | 区分 caller abort、网络错误和 server Retry-After，不自动无限重试 |
| SQLite busy | 检查产品 writer/transaction/instance lock；Foundation busy timeout 不是并发架构 |
| checkpoint busy | 保留错误并等待安全窗口，不把 busy 当成功 |

## 5. 依赖与供应链

Cargo.lock 与 pnpm lock 必须提交。GitHub Actions 使用 pinned SHA、固定 runner、最小权限、job timeout 和
不持久化 checkout credential。依赖更新单独评审 API、安全公告、license、bundle/compile 变化，并在
所有消费者验证。

## 6. 安全事件

Foundation 通常不持有生产数据，但依赖漏洞会传播到多个产品。确认受影响 package/API 和消费者，
优先发布新的不可变当前版本，再让每个产品重建自身制品；不要用 runtime CDN 热替换。公开报告不得
包含消费者生产配置或数据，漏洞使用私密渠道。仅当前版本和当前 `main` 接受修复。
