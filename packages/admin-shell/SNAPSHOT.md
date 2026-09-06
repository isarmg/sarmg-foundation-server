# 当前 Shell 展示层快照

源码提供亮/暗图标切换，以及空 navigation 时全宽呈现产品实例工作区；诊断入口、面板和专用前端解析器已移除。
会话、认证 client、CSRF 和管理员合同未替换，消费者仍使用锁定的已发布 0.6.0 依赖。

五个产品统一使用本源码编译快照；Dufs 只分发原生 Web 模块，不引入 React。公共默认值、消费者覆盖和产品对象映射见 [工作区配置](../../docs/admin-workspace.md)。不得修改已发布的 npm tarball，
也不得在同一产品混用两个 Shell Context。更新方法：

```sh
pnpm --filter @sarmg/admin-shell build
node scripts/sync-admin-shell.mjs /absolute/product
```

生成的 `clients/web/shell/` 包含 JS、类型和 SHA-256 清单，产品构建验证摘要。
所有产品组件从该快照引用 Shell，不要求产品构建时存在同级 Foundation 仓库。
更改仍需在下一次正式版本发布中统一收敛，不覆盖既有不可变版本。
