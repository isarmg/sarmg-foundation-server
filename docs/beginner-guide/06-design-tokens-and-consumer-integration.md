# 06. Design Tokens 与消费者集成

## 6.1 提供内容

`@sarmg/design-tokens` 导出 TypeScript token 对象、light CSS 和 dark CSS。当前仅包含被真实消费者使用的
颜色、间距和圆角，不是通用设计系统。

## 6.2 CSS custom property

变量统一使用 `--sarmg-*`。消费者在自身入口导入基础 CSS，并按产品主题策略选择 dark 文件。包不加载
字体、不设置全局 reset、不切换 DOM class，也不在运行时访问 CDN。

## 6.3 TypeScript token

程序化样式/图表可导入 token 对象，但应避免把颜色值复制成产品常量。CSS 与 TS 来源需在测试中保持
一致；改变 token 是可见破坏，应验证所有消费者。

## 6.4 消费者所有权

产品拥有布局、组件、可访问性、响应式、主题状态和品牌。Foundation 只给 primitive，不把多个产品拉进
同一个 UI release cadence。

## 6.5 集成步骤

锁定精确 package 版本；显式 package dependency；导入需要的 export；构建产品；检查 package 内容已被
编入制品；离线运行证明不依赖 registry；跑视觉/可访问性与产品测试。

## 6.6 Dark mode

dark CSS 只覆盖需要变化的 token。产品决定 `prefers-color-scheme`、用户设置或固定主题。避免在共享包中
写脚本读取 localStorage，从而侵入产品状态。

## 6.7 新 token 准入

至少两个真实消费者使用相同语义；名称描述用途而非某页面；light/dark 有明确值；对比度由消费者情境
验证；TypeScript/CSS/test/docs 同步。

## 6.8 删除与重命名

当前 0.x 可直接删除或重命名，更新所有消费者后发布新版本。不要保留重复 custom property alias，否则
CSS 兼容面会永久增长且难以搜索。

## 6.9 练习

创建最小 HTML 同时导入 light/dark exports，检查 computed values，再断网运行。确认没有全局元素样式或
外部资源请求。
