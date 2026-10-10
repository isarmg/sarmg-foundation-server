# @xcss/web/design-tokens

唯一 `@xcss/web@1.0.0` 包的 `design-tokens` 公开子路径提供管理 Web 的最小设计原语，不承担通用组件库职责。它公开：

- TypeScript `tokens` 与 `semanticTokens.light/dark`；
- `tokens.css` 与 `tokens.dark.css`；
- 只在 `[data-xcss-scope]` 内生效的 `reset.css`；
- 支持键盘焦点、减少动画、强制色彩及视觉隐藏的 `accessibility.css`。

```css
@import "@xcss/web/design-tokens/tokens.css";
@import "@xcss/web/design-tokens/tokens.dark.css";
@import "@xcss/web/design-tokens/reset.css";
@import "@xcss/web/design-tokens/accessibility.css";
```

应用根节点必须显式添加 `data-xcss-scope`；暗色覆盖要求同一作用域设置 `data-theme="dark"`。重置样式不
修改作用域外页面，包也不执行脚本、读取浏览器存储或请求 CDN。TypeScript 语义值必须与两份 CSS 的最终
值逐字节一致；清理后的构建会阻止已删除的设计令牌残留在发行归档中。

产品仍拥有品牌色、组件、布局、响应式、字体、主题选择/持久化、交互状态和完整可访问性/视觉回归测试。
新增设计令牌按产品中立职责、实际语义和可访问性边界判断；真正通用的机制暂时只有一个消费者也可以共享。当前版本不保留 CSS 别名或过渡变量。
