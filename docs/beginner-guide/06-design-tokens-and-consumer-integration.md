# 06. Design Tokens 与真实消费者集成

## 6.1 它不是组件库

`@sarmg/design-tokens`只提供少量被多个产品实际共享的primitive与浏览器基线。它不包含Button、Modal、
Table、路由、图标、品牌logo、字体、主题store或页面布局。保持这一边界可以让产品独立调整UI，而不会因
一个页面需求强迫所有项目升级同一组件库。

## 6.2 四个CSS入口

```css
@import "@sarmg/design-tokens/tokens.css";
@import "@sarmg/design-tokens/tokens.dark.css";
@import "@sarmg/design-tokens/reset.css";
@import "@sarmg/design-tokens/accessibility.css";
```

- `tokens.css`定义primitive与light semantic custom properties；
- `tokens.dark.css`只在dark selector下覆盖变化的semantic值；
- `reset.css`只作用于显式scope；
- `accessibility.css`提供focus/reduced-motion/forced-colors/visually-hidden基线。

应用根节点：

```html
<div id="root" data-sarmg-scope data-theme="dark"></div>
```

主题是light时可以不设置`data-theme`或使用产品定义的当前状态。Foundation不读取系统偏好或浏览器storage，
产品决定主题来源和持久化。

## 6.3 Scoped reset

全局reset可能破坏嵌入页面、第三方内容或Dufs前端，所以选择`[data-sarmg-scope]`。box-sizing、表单font、
disabled cursor、heading wrap等只在scope内生效。产品忘记加attribute时不会获得reset，这是显式opt-in的
成本，也是防止全局污染的保障。

测试必须确认scope外computed style不变，不能只搜索CSS文本里是否有变量。

## 6.4 可访问性最低基线

### Focus visible

键盘导航时需要明显焦点，不要用`outline: none`覆盖。产品组件仍要检查focus order、modal trap和关闭后
焦点恢复。

### Reduced motion

`prefers-reduced-motion: reduce`限制animation和transition。它不是“自动让所有产品可访问”，产品JavaScript
动画、video和canvas仍需自己的处理。

### Forced colors

高对比度模式使用系统Highlight保证focus可见。产品自定义图表、status颜色和图标仍需在真实OS模式测试。

### Visually hidden

utility让文本视觉隐藏但保留给辅助技术，适合图标按钮label等。不要用`display:none`替代，因为那会从
accessibility tree删除。

## 6.5 TypeScript token

```ts
import { tokens, semanticTokens } from "@sarmg/design-tokens";

const chartGap = tokens.space[3];
const fallbackText = semanticTokens.dark.textPrimary;
```

TS对象用于无法解析CSS custom property的build-time代码或图表。semantic TS值必须与light CSS加dark override
后的effective值一致。不要复制hex到产品常量；若语义只属于一个产品，留在产品CSS。

## 6.6 新token准入

逐项回答：

1. 哪两个真实消费者使用相同语义而非刚好相同颜色？
2. 名称描述用途还是某页面/品牌？
3. light与dark的effective值是什么？
4. 产品情境下对比度和forced-colors如何验证？
5. 删除后消费者具体需要复制什么？

例如`--sarmg-color-bg-panel`是跨页面语义候选；`--sentinel-camera-offline-card`明显属于Sentinel。

## 6.7 消费者采用组件

非Dufs Web通常显式依赖：

- `@sarmg/admin-web`；
- `@sarmg/contracts`；
- `@sarmg/http-client`；
- `@sarmg/design-tokens`；
- 精确React/React DOM和Vite/TypeScript peers。

本地联调可暂用`file:../../../sarmg-foundation/packages/...`。Foundation release后必须替换为GitHub Release
中不可变tgz URL，重建`package-lock.json`，再把整个产品复制到没有sibling Foundation的checkout验证。

## 6.8 Rust消费者采用组件

按最小需要选crate。一个只需Schema fingerprint的rusqlite工具不应引入`sarmg-sqlite`；客户端crate不应
引入`sarmg-server-target`；只做错误输出的service adapter可以仅用`sarmg-error`。

本地path联调后改成Foundation release commit完整rev与`version="=0.3.1"`。Git branch、短SHA和永久path都
不能提供不可变来源。

## 6.9 产品边界不能在接入时丢失

接入共享组件是“替换相同primitive”，不是删除产品加强规则：

- Sunshine仍必须验证上游TLS证书；
- Server release仍检查产品specific路径、mode、ELF和self-binding；
- SQLite打开前仍做no-follow、owner/mode和实例锁；
- Sentinel仍验证MediaMTX companion；
- Media仍管理文件树两阶段提交；
- Host Agent仍维护跨平台spool与配对协议。

若共享helper比产品旧实现弱，应缩小采用范围或加强Foundation，而不是降低产品测试。

## 6.10 Dufs例外怎样接入

Dufs不使用`admin-web`、React/Vite或design tokens作为重写前端的理由。它可以直接采用Rust admin-auth、
contracts、error、schema、sqlite、server-target，并在原生ES modules中严格验证同一个AdministratorSession。
这叫“共享合同、保留产品交付形态”。

## 6.11 删除与重命名

删除CSS变量或TS export时，在新当前版本直接删除并同步全部消费者；不要保留双变量alias。clean build与
tar inventory必须证明旧产物不在dist/tgz。若消费者仍使用旧名称，升级其源码，而不是用CSS fallback把
两套语义长期并存。

## 6.12 本章练习

1. 在最小HTML中验证scope内外box-sizing差异。
2. 列出产品品牌变量与Foundation semantic token各三个例子。
3. 用浏览器模拟dark、reduced motion和forced colors，记录仍需产品负责的缺口。
4. 为一个consumer写本地file阶段与不可变tgz阶段的检查清单。
5. 解释为什么Dufs不迁React仍可算完成Foundation认证统一。
