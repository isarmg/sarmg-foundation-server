# 07. 版本、软件包、发行与破坏性变更

## 7.1 当前版本模型

单个 Rust crate `xcss` 与单个 npm 包 `@xcss/web` 统一为 `1.0.0`。内部模块和子路径共同构建、测试、发布。每个版本是
不可变合同，一旦tag/asset公开就不移动、不覆盖、不用同版本重新打包。

以下都可能是breaking change：

- 删除/重命名Rust函数、type、error variant或npm 导出入口；
- 改管理员 username、密码、Argon2、令牌、header 或 CSRF 规则；
- 改AdministratorSession字段/role/path；
- 改JSON guard/Schema/fixture；
- 改Schema 结构指纹或SQLite PRAGMA；
- 改timeout/body limit/retry/error语义；
- 改React/Vite/Node精确基线；
- 改CSS token/reset selector；
- 改release-tree/path/mode/size规则。

## 7.2 不保留兼容层

新当前版本只保留一个名字和行为。删除旧function、field、软件包导出、CSS property、Schema version和
消费者调用；不添加已弃用的封装、别名、dual 解析器、版本协商或“参数不同也试一下”的回退。

## 7.3 软件包 build不是源码复制

唯一 Web 软件包的流程：clean dist→tsc→复制公开静态文件→从dist测试→pack→tar审计→空目录离线安装。

`package.json#files=["dist"]`限制发布面，导出入口限制公共入口。README、LICENSE和软件包元数据由npm
规则随包携带；src/test/node_modules不进入。若一个导出入口只在工作区能解析，它不算实现。

## 7.4 对等依赖所有权

contracts、http-client、admin-web 等均为同一 `@xcss/web` 包的内部模块，通过公开子路径自引用。
它们没有独立 peer、版本或发行归档。外部 React/Vite 依赖保留精确 peer，消费者在自己的锁文件中固定。
源码与发布清单均不得含内部 `workspace:`、`file:` 或独立 `@xcss/*` 包依赖。

## 7.5 Rust消费版本

正式产品不依赖浮动分支或本地path：

```toml
xcss = {
  git = "https://github.com/isarmg/xcss.git",
  rev = "<v1.0.0 对应完整 40 位 commit>",
  version = "=1.0.0"
}
```

完整rev绑定Git对象，精确的 version核对单体软件包身份。更新xcss时明确改rev/version并重建
Cargo.lock；不要只改lock让源码看不出依赖变化。

## 7.6 Web消费版本

发布前可用本地`file:`验证跨仓库实现；发布后改成GitHub 发行 tgz URL，重建`package-lock.json`。最终
测试应在没有`../xcss`的独立源码检出运行，证明软件包不是靠同级路径工作。

消费者使用npm；无需把产品迁成pnpm。锁文件与`npm ci`是产品自己的可复现合同。

## 7.7 发行资产怎样生成

```text
工作树完全干净 + tag精确指向HEAD
├─ package_release -> 1个 xcss-web-1.0.0.tgz
├─ deterministic tool bundle
├─ state-contract.json
├─ hash(state contract) -> release-identity.json
├─ build-inventory.json
├─ SHA256SUMS
└─ release-tree.json（在artifacts树外）
```

xcss无runtime状态，所以状态合同的schema为null，资源/锁/外部要求/companion为空。identity target
固定为 `x86_64-unknown-linux-gnu`。xcss 的 Rust crate 和 npm 构建进程均受同一服务端平台边界约束。

## 7.8 确定性工具归档

release-tool tar只包含固定allowlist文件，source必须普通单链接且不超过上限。tar member固定mtime=0、
uid/gid=0、owner/group=root、mode和排序；gzip header也固定。相同source应生成相同字节，便于独立复核。

## 7.9 发行目录清单与 SHA256SUMS

`SHA256SUMS`覆盖所有其他artifact但不覆盖自己。release-tree描述artifacts目录中每个文件的path、mode、size、
SHA，并要求没有额外文件。清单不描述自己，避免递归hash。

verifier拒绝absolute/parent/backslash/NUL path、link/special file、超限树和TOCTOU替换。产品仍要在此后执行
产品specific目录、权限、ELF、binary self-binding和总量规则。

## 7.10 GitHub Actions权限

普通CI顶层permissions为空，job只读contents。唯一写权限位于精确发行 workflow的唯一发行 job，且
trigger只有`v*`标签 push。action锁完整SHA，源码检出不持久凭据，runner/Node/timeout固定。

这不是为了“让YAML更整齐”，而是限制不可信源码和第三方action接触发行权限。

## 7.11 发布前后消费者闭环

```text
发布前
xcss代码/测试 -> 产品本地path/file联调 -> 保存实际失败和通过结果

发布
immutable tag + assets

发布后
消费者rev/tgz + rebuilt lock -> independent checkout -> full CI/release/offline
 -> 产品commit -> 对应正式CI、资产与实际运行验证
```

只完成任一半都不够：没有发布前联调可能发布错误抽象；没有发布后验证则无法证明不可变依赖真实可用。

## 7.12 回退策略

若新xcss版本有问题，不修改已发布版本。消费者可以在源码依赖层回退到此前不可变版本并重建产品，
同时开发新的修复版本。运行时不同时加载两代xcss做兼容；软件包版本不能代替持久状态身份检查。

## 7.13 发布证据

保留：完整commit、annotated 标签、workflow run、工具链、两个lock hash、软件包归档 hash、发行身份、
状态合同、inventory、release-tree、消费者最终commit和验证结果。不要把npm/GitHub 令牌、产品Secret、
真实数据库或用户信息放入证据。

## 7.14 本章练习

1. 对“把password最小长度从12改成16”列出所有需要同步的层。
2. 解释为何合并后内部模块不再声明软件包依赖，如何从单个包的公开子路径导入。
3. 画出状态合同、发行身份、SHA256SUMS和release-tree的hash关系。
4. 为一次失败发行写处理方案，要求不移动tag/覆盖asset。
5. 说明产品本地path构建通过为何仍不能代替正式Git/tgz来源的独立构建和发行验收。
