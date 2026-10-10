# 默认字体：英文 Normal NL 正体 / 中文 / 日文

- 西文使用官方 v7.9 Maple Mono Normal NL 的 Regular/Bold WOFF2，字形为普通直立、非手写体，字体元数据 italicAngle 为 0，不以 CSS 抵消斜体。
- `calt`、`liga`、`clig`、`dlig` 关闭，不另行启用手写风格特性；关闭字体合成。英文粗体有独立 Bold 文件。
- CJK 使用官方 Maple Mono NL CN v7.9 的 Regular/Bold 正体，保留该字体提供的中文、日文及西文字体未覆盖的其他字符。并非承诺覆盖 Unicode 的所有汉字。
- 字体仍按不重叠 Unicode 范围生成 WOFF2，每个资源小于 256 KiB，保持 原生与 React 运行形态的资源预算。
- 管理 Web 首次访问调用 `startAfterFonts()`：显式加载并解码所有 Regular/Bold 字体，包括尚未使用的 CJK 分片和 Bootstrap 字体。完成后再挂载页面，页面内切换菜单不再触发字体请求。
- HTML 在 `head` 中内嵌 `boot.css`，并设置 `data-xcss-fonts="pending"`；准备期间只显示与系统主题一致的纯色背景。没有超时或加载失败后的系统字体回退，失败时保持背景，重新载入页面可重试。
- CSS 统一使用 `font-display:block`。字体加载 Promise 在同一文档内复用，所有入口、登录页和后续菜单使用同一组已解码字体。

## 可复现工具链

在隔离 Python 环境安装 `fonttools[woff]==4.60.1`、`brotli==1.1.0`，将 `provenance.json` 中指定的官方 ZIP 下载到显式路径。在新的、空 `cjk/` 输出目录运行：

```sh
python web/web-fonts/scripts/build-cjk.py /absolute/MapleMonoNL-CN-unhinted.zip
python web/web-fonts/scripts/split-cjk.py
python web/web-fonts/scripts/use-normal-latin.py /absolute/MapleMonoNormalNL-Woff2.zip
python web/web-fonts/scripts/build-bootstrap.py
node web/web-fonts/scripts/build.mjs
```

生成器逐片检查实际 cmap，分片不得丢字；Native 预算细分只替换本次生成且摘要匹配的中间字体，不处理用户文件。

## 当前发行分发

字体通过唯一正式 `@xcss/web` 包的 `web-fonts` 公开子路径分发。消费者固定不可变发行包，并在构建时校验来源、摘要和许可证。
管理 Web 通过包依赖使用 Normal NL 正体与 CJK 资源。
消费者导入包的 CSS，构建时校验来源、字体摘要和许可证，运行时只加载产品同源资源，不需要同级 xcss 工作区或 CDN。

更新字体必须发布新的不可变包、更新消费者锁图并重建资源合同，不能覆盖旧归档或伪造原生平台验收。
随包保留对应许可证；客户端本机 Web 不属于此次服务端管理 Web 范围。

已使用审核过的字体源码快照的服务端同步 CSS、启动模块、内嵌背景样式及对应摘要，保留已审核的字体二进制。Xczs 的过渡 CSS 引用固定发行包中的字体文件，由构建检查验证构建资源路径的差异。`startup.json` 固定启动模块和背景样式摘要；消费者构建不读取同级 xcss 工作区。

使用冷缓存并阻塞 Bootstrap 和稀有 CJK 字体，验证网站仅显示纯色背景，超过原来的 1.2 秒也不显示系统字体；释放所有字体后再进入页面，检查全部字体已解码、后续字符和菜单不产生新字体请求：

```sh
node scripts/check-font-navigation.mjs web/web-fonts ../xsos ../xscs ../xcos ../xszs ../xczs
```
