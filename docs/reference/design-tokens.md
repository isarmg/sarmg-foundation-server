# 设计令牌

`@xcss/web/design-tokens` 提供 CSS/TypeScript 原语；组件、外壳和字体由同包的其他子路径提供。

## 四个CSS入口

```css
@import "@xcss/web/design-tokens/tokens.css";
@import "@xcss/web/design-tokens/tokens.dark.css";
@import "@xcss/web/design-tokens/reset.css";
@import "@xcss/web/design-tokens/accessibility.css";
```

- `tokens.css`定义原语与light semantic custom properties；
- `tokens.dark.css`只在dark selector下覆盖变化的semantic值；
- `reset.css`只作用于显式作用域；
- `accessibility.css` 提供键盘焦点、减少动画、强制色彩与视觉隐藏的基线。

应用根节点：

```html
<div id="root" data-xcss-scope data-theme="dark"></div>
```

主题是light时可以不设置`data-theme`或使用产品定义的当前状态。设计令牌本身不读取系统偏好或浏览器 storage；admin-shell 负责系统主题与 `sarmg:theme` 的选择，
产品决定主题来源和持久化。

## 作用域内的重置样式

全局reset可能破坏嵌入页面、第三方内容或Xczs前端，所以选择`[data-xcss-scope]`。box-sizing、表单font、
disabled cursor、heading wrap等只在作用域内生效。产品忘记加attribute时不会获得reset，这是显式opt-in的
成本，也是防止全局污染的保障。

测试必须确认作用域外computed style不变，不能只搜索CSS文本里是否有变量。

### 可见焦点

键盘导航时需要明显焦点，不要用`outline: none`覆盖。产品组件仍要检查焦点 order、modal trap和关闭后
焦点恢复。

### 减少动画

`prefers-reduced-motion: reduce`限制animation和transition。它不是“自动让所有产品可访问”，产品JavaScript
动画、video和canvas仍需自己的处理。

### 强制色彩

高对比度模式使用系统Highlight保证焦点可见。产品自定义图表、status颜色和图标仍需在真实OS模式测试。

### 视觉隐藏

utility让文本视觉隐藏但保留给辅助技术，适合图标按钮label等。不要用`display:none`替代，因为那会从
accessibility tree删除。

## TypeScript 令牌

```ts
import { tokens, semanticTokens } from "@xcss/web/design-tokens";

const chartGap = tokens.space[3];
const fallbackText = semanticTokens.dark.textPrimary;
```

TS对象用于无法解析CSS custom property的构建期代码或图表。semantic TS值必须与light CSS加dark override
后的effective值一致。不要复制hex到产品常量；若语义只属于一个产品，留在产品CSS。

产品专有语义保留在产品 CSS。公共令牌变更同时验证 light/dark、键盘焦点、reduced-motion 与 forced-colors。完整管理页面接入见[使用公共模块](../usage.md)。
