# 默认字体：英文 Normal NL 正体 / 中文 / 日文

- 西文使用官方 v7.9 Maple Mono Normal NL 的 Regular/Bold WOFF2，字形为普通直立、非手写体，字体元数据 italicAngle 为 0，不以 CSS 抵消斜体。
- `calt`、`liga`、`clig`、`dlig` 关闭，不另行启用手写风格特性；关闭字体合成。英文粗体有独立 Bold 文件。
- CJK 使用官方 Maple Mono NL CN v7.9 的 Regular/Bold 正体，保留该字体提供的中文、日文及西文字体未覆盖的其他字符。并非承诺覆盖 Unicode 的所有汉字。
- 按不重叠 Unicode 范围生成 WOFF2，浏览器按需加载；每个资源小于 256 KiB，保持 Native/React Profile 原有预算。
- 源码字体归档、输出字体、CSS 和许可证的 SHA-256 记录在 `provenance.json`。Normal NL 许可证为 `NORMAL-LICENSE.txt`，CJK 许可证保持 `CJK-LICENSE.txt`。

## 可复现工具链

在隔离 Python 环境安装 `fonttools[woff]==4.60.1`、`brotli==1.1.0`，将 provenance 指定的官方 ZIP 下载到显式路径。在新的、空 `cjk/` 输出目录运行：

```sh
python packages/web-fonts/scripts/build-cjk.py /absolute/MapleMonoNL-CN-unhinted.zip
python packages/web-fonts/scripts/split-cjk.py
python packages/web-fonts/scripts/use-normal-latin.py /absolute/MapleMonoNormalNL-Woff2.zip
node packages/web-fonts/scripts/build.mjs
```

生成器逐片检查实际 cmap，分片不得丢字；Native 预算细分只替换本次生成且摘要匹配的中间字体，不处理用户文件。

## 当前源码分发

已发布 Foundation 0.6.0 保持不变。本次没有发布或冒充新的 npm 版本。新的字体源快照通过显式命令同步到各产品 `clients/web/fonts/`，作为应纳入 Git 的构建资源：

```sh
node scripts/sync-default-fonts.mjs /absolute/product-repository
```

这些是同一上游的生成资源，不是产品自行维护的字体规则，也不是旧版兼容入口。产品构建校验快照摘要后直接引用其 CSS，不再加载旧字体 CSS；已锁定的 Foundation npm/Rust 依赖不被修改。独立克隆产品仓库即可构建，不需要同级 Foundation 工作区或运行时 CDN。

本次 Normal NL 已同步 Sunshine Manager 和 Host Monitoring；其他产品尚未同步此次字体变更。
后续正式发布应将这些新资源纳入各产品新制品及静态合同，随包保留相应字体许可证；不得覆盖先前发行归档。Agent 本机 Web 不属于本次 Server 管理 Web 的范围。
