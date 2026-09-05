# @sarmg/design-tokens

`@sarmg/design-tokens@0.6.0` 是 Sarmg 管理 Web 的最小设计 primitive，而不是通用组件库。它公开：

- TypeScript `tokens` 与 `semanticTokens.light/dark`；
- `tokens.css` 与 `tokens.dark.css`；
- 只在 `[data-sarmg-scope]` 内生效的 `reset.css`；
- 键盘 focus、reduced motion、forced-colors 和 visually-hidden 的 `accessibility.css`。

```css
@import "@sarmg/design-tokens/tokens.css";
@import "@sarmg/design-tokens/tokens.dark.css";
@import "@sarmg/design-tokens/reset.css";
@import "@sarmg/design-tokens/accessibility.css";
```

应用根节点必须显式添加 `data-sarmg-scope`；暗色覆盖要求同一作用域设置 `data-theme="dark"`。reset 不
修改作用域外页面，包也不执行脚本、读取浏览器存储或请求 CDN。TypeScript 语义值必须与两份 CSS 的最终
值逐字节一致，clean build 会阻止已删除 token 残留在 tarball。

产品仍拥有品牌色、组件、布局、响应式、字体、主题选择/持久化、交互状态和完整可访问性/视觉回归测试。
新增 token 必须证明至少两个真实消费者具有同一语义；当前版本不保留 CSS alias 或过渡变量。
