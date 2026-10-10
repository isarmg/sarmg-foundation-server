# 06. 设计令牌与真实消费者集成

## 6.1 它不是组件库

`@xcss/web/design-tokens`只提供少量被多个产品实际共享的原语与浏览器基线。它不包含Button、Modal、
Table、路由、图标、品牌logo、字体、主题store或页面布局。保持这一边界可以让产品独立调整UI，而不会因
一个页面需求强迫所有项目升级同一组件库。

## 6.2 四个CSS入口

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

## 6.3 作用域内的重置样式

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

## 6.5 TypeScript 令牌

```ts
import { tokens, semanticTokens } from "@xcss/web/design-tokens";

const chartGap = tokens.space[3];
const fallbackText = semanticTokens.dark.textPrimary;
```

TS对象用于无法解析CSS custom property的构建期代码或图表。semantic TS值必须与light CSS加dark override
后的effective值一致。不要复制hex到产品常量；若语义只属于一个产品，留在产品CSS。

## 6.6 新令牌准入

逐项回答：

1. 是否为产品中立的设计语义，实际调用点与安全、可访问性边界是什么？消费者数量不作为准入门槛。
2. 名称描述用途还是某页面/品牌？
3. light与dark的effective值是什么？
4. 产品情境下对比度和forced-colors如何验证？
5. 删除后消费者具体需要复制什么？

例如`--xcss-color-bg-panel`是跨页面语义候选；`--xcos-camera-offline-card`明显属于xcos。

## 6.7 消费者采用组件

产品只在 npm 依赖中声明唯一的 `@xcss/web` 发行包，再通过以下公开子路径导入：

- `@xcss/web/admin-web`；
- `@xcss/web/contracts`；
- `@xcss/web/http-client`；
- `@xcss/web/design-tokens`；

React/React DOM 与 Vite 外部 peer 由消费者显式固定精确版本；TypeScript 和类型包作为产品开发依赖
使用相同精确基线，它们不是本包声明的 peer。

本地联调可暂用`file:../../../xcss`。xcss 发行后必须替换为GitHub 发行
中不可变tgz URL，重建`package-lock.json`，再把整个产品复制到没有同级 xcss的源码检出验证。

## 6.8 Rust消费者采用组件

服务端统一声明一个 `xcss` crate，再按需要导入 `xcss::<module>`。这些模块共享软件版本、源码修订号与 Linux AMD64 GNU 编译边界。

本地path联调后改成xcss 发行 commit完整rev与`version="=1.0.0"`。Git 分支、短SHA和永久path都
不能提供不可变来源。

## 6.9 产品边界不能在接入时丢失

接入共享组件是“替换相同原语”，不是删除产品加强规则：

- xscs 仍必须验证 Sunshine 上游 TLS 证书；
- 服务端发行仍检查产品specific路径、mode、ELF和self-binding；
- SQLite打开前仍做不跟随符号链接的、owner/mode和实例锁；
- xcos仍验证MediaMTX companion；
- xszs 仍管理备份文件树的两阶段提交；
- xsoc仍维护跨平台spool与配对协议。

若共享辅助函数比产品旧实现弱，应缩小采用范围或加强xcss，而不是降低产品测试。

## 6.10 Xczs 原生业务模块怎样接入

Xczs 当前采用 `web-react-admin`，通过 `@xcss/web/admin-shell` 和 React/Vite 提供登录、导航及页面骨架。
原生 ES 模块保留文件列表与上传业务，并在独占 DOM 区域和共享外壳组合；不复制认证客户端。
Rust 依赖仍是唯一 `xcss` crate，全部正式 Web 资源与可执行文件一起构建。

## 6.11 删除与重命名

删除CSS变量或TS 导出入口时，在新当前版本直接删除并同步全部消费者；不要保留双变量别名。清理后构建与
tar inventory必须证明旧产物不在dist/tgz。若消费者仍使用旧名称，升级其源码，而不是用CSS 回退把
两套语义长期并存。

## 6.12 本章练习

1. 在最小HTML中验证作用域内外box-sizing差异。
2. 列出产品品牌变量与xcss semantic 令牌各三个例子。
3. 用浏览器模拟dark、reduced motion和forced colors，记录仍需产品负责的缺口。
4. 为一个消费者写本地file阶段与不可变tgz阶段的检查清单。
5. 解释 Xczs 的 React 管理外壳和原生文件业务模块如何共享一套认证与构建合同。
