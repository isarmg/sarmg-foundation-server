# 08. 测试、调试与新增共享能力

## 8.1 完整门禁

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

## 8.2 测试层次

Rust 单元测试 primitive 与错误边界；Web unit 验证 guard/client；package test 验证 dist/exports；消费者
集成测试验证真实产品语义。四层不能互相替代。

## 8.3 新能力提案

列出至少两个当前消费者、重复代码、相同语义、不同之处、安全最强边界、最小 API、失败模型、负例、
版本策略、退出方案。若无法填写，先留在产品。

## 8.4 调试方法

先判断源码、构建输出、package export、锁图还是消费者 bundler/feature 问题。用最小临时消费者复现，
不要直接在产品中添加路径 hack 或 alias。

## 8.5 合同测试

每个正例至少配 unknown field、边界、错误类型和过大输入负例。Rust/TS/JSON Schema 共享语义时使用固定
跨语言 fixture，并断言两端都拒绝相同非法结构。

## 8.6 依赖变更

单独评审公告、license、MSRV/Node、bundle/compile、feature/default 与消费者影响。更新锁后跑所有消费者，
不把依赖更新混进无关功能提交。

## 8.7 名称变更审计

搜索目录、Cargo/package 名、lock、scope、import、CSS property、Schema example、test fixture、docs、CI
和仓库 remote。当前身份必须唯一，不发布 alias package。

## 8.8 提交检查

无 target/node_modules/临时 tarball；dist 政策一致；文档链接与命令有效；包内容最小；所有质量门和真实
消费者验证完成；一个大问题一个提交。
