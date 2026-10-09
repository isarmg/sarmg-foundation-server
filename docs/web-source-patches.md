# 已发布 Web 包的源码修复

公共组件在 xcss 源码中修复，消费者仍固定官方不可变 release URL 和 lockfile integrity。正式发行尚未包含的已审阅修复，通过 `scripts/export-consumer-web-patches.mjs` 生成两个明确文件的源码补丁：单包内账号组件 `dist/admin-shell/account.js` 和默认外观 `dist/admin-ui/content-blocks.css`。这条管道不复制整个 Shell/UI/context，也不改变发布制品。

当前单体已经包含工作区的账号页面和默认外观修改；本次消费清单的修改前后摘要相同、编辑列表为空，只验证身份，不重复修改正式包字节。未来尚未发布的已审阅修复仍可通过此工具导出。

先编译公共包，使用经锁定安装恢复的原始 1.0.0 消费者作为基线，显式传入每个消费者的 package 根目录：

```sh
node scripts/export-consumer-web-patches.mjs --baseline <原始消费者 package 根目录> <目标 package 根目录>...
```

导出器校验已审阅原始文件的 SHA-256，生成自包含 `scripts/apply-xcss-patches.mjs` 和 `patches/xcss.json`。清单记录源码 SHA-256、原发行身份、修改前后 SHA-256 和最小编辑。消费者独立检出无需 xcss 相邻目录。

React 消费者在 `prebuild` 和 `predev` 应用补丁；xczs 的构建脚本直接调用加载器。构建器先执行 `npm ci --ignore-scripts`，随后运行显式构建入口，修复仍会自动生效。所有文件先完成版本、锁身份、基线和输出校验，再写入；已应用状态可重复验证，未知文件或升级版本直接拒绝，不覆盖本地修改。

公共测试 `pnpm run test:consumer-patches` 检查应用、幂等、未知基线、升级身份、损坏补丁和链接目标。升级到包含修复的新正式发行版后，验证产品行为，再统一删除消费者补丁、加载器和构建入口；既有发行保持不变。
