# 默认字体：英文 Normal NL 正体 / 中文 / 日文

- 西文使用官方 v7.9 Maple Mono Normal NL 的 Regular/Bold WOFF2，字形为普通直立、非手写体，字体元数据 italicAngle 为 0，不以 CSS 抵消斜体。
- `calt`、`liga`、`clig`、`dlig` 关闭，不另行启用手写风格特性；关闭字体合成。英文粗体有独立 Bold 文件。
- CJK 使用官方 Maple Mono NL CN v7.9 的 Regular/Bold 正体，保留该字体提供的中文、日文及西文字体未覆盖的其他字符。并非承诺覆盖 Unicode 的所有汉字。
- 按不重叠 Unicode 范围生成 WOFF2，浏览器按需加载；每个资源小于 256 KiB，保持 Native/React Profile 原有预算。
- 首屏使用独立的 `Sarmg Maple Bootstrap` Regular/Bold 子集。它合并完整西文与管理 Web 的静态 UI 汉字，使首屏每个字重只需一个字体请求；其余字符回退到完整 `Sarmg Maple` 分片。
- 源码字体归档、输出字体、CSS 和许可证的 SHA-256 记录在 `provenance.json`。Normal NL 许可证为 `NORMAL-LICENSE.txt`，CJK 许可证保持 `CJK-LICENSE.txt`。

## 可复现工具链

在隔离 Python 环境安装 `fonttools[woff]==4.60.1`、`brotli==1.1.0`，将 provenance 指定的官方 ZIP 下载到显式路径。在新的、空 `cjk/` 输出目录运行：

```sh
python packages/web-fonts/scripts/build-cjk.py /absolute/MapleMonoNL-CN-unhinted.zip
python packages/web-fonts/scripts/split-cjk.py
python packages/web-fonts/scripts/use-normal-latin.py /absolute/MapleMonoNormalNL-Woff2.zip
python packages/web-fonts/scripts/build-bootstrap.py
node packages/web-fonts/scripts/build.mjs
```

生成器逐片检查实际 cmap，分片不得丢字；Native 预算细分只替换本次生成且摘要匹配的中间字体，不处理用户文件。

## 当前发行分发

字体通过正式 `@sarmg/web-fonts` Release 包分发。消费者固定不可变 Release 包，并在构建时校验来源、摘要和许可证。
五个管理 Web 均使用 Normal NL 正体与 CJK 资源；不再将字体源码快照同步到产品 `clients/web/fonts/`。
消费者导入包的 CSS，构建时校验来源、字体摘要和许可证，运行时只加载产品同源资源，不需要同级 Foundation 工作区或 CDN。

更新字体必须发布新的不可变包、更新消费者锁图并重建资源合同，不能覆盖旧归档或伪造原生平台验收。
随包保留对应许可证；Client 本机 Web 不属于此次 Server 管理 Web 范围。
首屏子集的发行与消费者验收记录见 0.7.5 Release 文档。
