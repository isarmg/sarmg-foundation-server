# 07. 版本、Package、Release 与破坏性变更

## 7.1 当前版本模型

22个crate和8个package统一为`0.5.0`。0.x允许破坏性演进，但“不稳定”不等于可以静默漂移：每个版本仍是
不可变合同，一旦tag/asset公开就不移动、不覆盖、不用同版本重新打包。

以下都可能是breaking change：

- 删除/重命名Rust函数、type、error variant或npm export；
- 改管理员 username、密码、Argon2、token、header 或 CSRF 规则；
- 改AdministratorSession字段/role/path；
- 改JSON guard/Schema/fixture；
- 改Schema fingerprint或SQLite PRAGMA；
- 改timeout/body limit/retry/error语义；
- 改React/Vite/Node精确基线；
- 改CSS token/reset selector；
- 改release-tree/path/mode/size规则。

## 7.2 不保留兼容层

新当前版本只保留一个名字和行为。删除旧function、field、package export、CSS property、Schema version和
consumer调用；不添加deprecated wrapper、alias、dual parser、版本协商或“参数不同也试一下”的fallback。

若涉及已发布持久状态：在线产品只实现target current；`sarmg-upgrade`在独立高权限离线工具中实现精确
source→target edge。开发期未发布数据可重建，不为它写升级代码。

## 7.3 Package build不是源码复制

每个package的流程：clean dist→tsc→复制公开静态文件→从dist测试→pack→tar审计→空目录离线安装。

`package.json#files=["dist"]`限制发布面，exports限制公共入口。README、LICENSE和package metadata由npm
规则随包携带；src/test/node_modules不进入。若一个export只在workspace能解析，它不算实现。

## 7.4 Peer dependency所有权

`@sarmg/http-client`把contracts作为精确peer；`admin-web`把contracts/http-client与React/Vite需要项作为精确
peer。这样消费者明确拥有依赖，不会因某个包内部悄悄嵌入另一版本而出现两份合同。

Foundation workspace中的`workspace:0.5.0`只用于dev/build。发布tar manifest不能含workspace协议；真实
consumer必须同时安装所需tgz。

## 7.5 Rust消费版本

正式产品不依赖浮动branch或本地path：

```toml
sarmg-contracts = {
  git = "https://github.com/isarmg/sarmg-foundation.git",
  rev = "<v0.5.0对应完整40位commit>",
  version = "=0.5.0"
}
```

完整rev绑定Git对象，exact version防止选错workspace package。更新Foundation时明确改rev/version并重建
Cargo.lock；不要只改lock让源码看不出依赖变化。

## 7.6 Web消费版本

发布前可用本地`file:`验证跨仓库实现；发布后改成GitHub Release tgz URL，重建`package-lock.json`。最终
测试应在没有`../sarmg-foundation`的独立checkout运行，证明package不是靠sibling路径工作。

消费者使用npm；无需把产品迁成pnpm。lockfile与`npm ci`是产品自己的可复现合同。

## 7.7 Release资产怎样生成

```text
工作树完全干净 + tag精确指向HEAD
├─ package_release -> 8个tgz
├─ deterministic tool bundle
├─ state-contract.json
├─ hash(state contract) -> release-identity.json
├─ build-inventory.json
├─ SHA256SUMS
└─ release-tree.json（在artifacts树外）
```

Foundation无runtime状态，所以state contract的schema为null，资源/锁/外部要求/companion为空。identity target
默认`source-any`；这不改变消费者Server只允许AMD64的规则。

## 7.8 Deterministic tool bundle

release-tool tar只包含固定allowlist文件，source必须普通单链接且不超过上限。tar member固定mtime=0、
uid/gid=0、owner/group=root、mode和排序；gzip header也固定。相同source应生成相同bytes，便于独立复核。

## 7.9 Release-tree与SHA256SUMS

`SHA256SUMS`覆盖所有其他artifact但不覆盖自己。release-tree描述artifacts目录中每个文件的path、mode、size、
SHA，并要求没有额外文件。manifest不描述自己，避免递归hash。

verifier拒绝absolute/parent/backslash/NUL path、link/special file、超限树和TOCTOU替换。产品仍要在此后执行
产品specific目录、权限、ELF、binary self-binding和总量规则。

## 7.10 GitHub Actions权限

普通CI顶层permissions为空，job只读contents。唯一写权限位于精确release workflow的唯一release job，且
trigger只有`v*`tag push。action锁完整SHA，checkout不持久credential，runner/Node/timeout固定。

这不是为了“让YAML更整齐”，而是限制不可信源码和第三方action接触release权限。

## 7.11 发布前后消费者闭环

```text
发布前
Foundation代码/测试 -> 消费者本地path/file -> integration-pending

发布
immutable tag + assets

发布后
消费者rev/tgz + rebuilt lock -> independent checkout -> full CI/release/offline
 -> consumer commit -> matrix passing
```

只完成任一半都不够：没有发布前联调可能发布错误抽象；没有发布后验证则无法证明不可变依赖真实可用。

## 7.12 回退策略

若新Foundation版本有问题，不修改已发布版本。消费者可以在源码依赖层回退到此前不可变版本并重建产品，
同时开发新的修复版本。运行时不同时加载两代Foundation做兼容；持久状态是否能回退由产品/Upgrade合同
单独判断，不能从package版本自动推断。

## 7.13 发布证据

保留：完整commit、annotated tag、workflow run、工具链、两个lock hash、package tar hash、release identity、
state contract、inventory、release-tree、消费者最终commit和验证结果。不要把npm/GitHub token、产品Secret、
真实数据库或用户信息放入证据。

## 7.14 本章练习

1. 对“把password最小长度从12改成16”列出所有需要同步的层。
2. 解释为什么`workspace:`可用于dev但不能出现在发布tar manifest。
3. 画出state contract、release identity、SHA256SUMS和release-tree的hash关系。
4. 为一次失败release写处理方案，要求不移动tag/覆盖asset。
5. 说明消费者本地path通过后为何仍不能把matrix标passing。
