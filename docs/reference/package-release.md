# 软件包与发行格式

构建和下载验证的操作步骤见[发行工具](../operations.md)。本页解释 npm 归档、构建身份和清单之间的关系。

## 软件包 build不是源码复制

唯一 Web 软件包的流程：clean dist→tsc→复制公开静态文件→从dist测试→pack→tar审计→空目录隔离安装。

`package.json#files=["dist"]`限制发布面，导出入口限制公共入口。README、LICENSE和软件包元数据由npm
规则随包携带；src/test/node_modules不进入。若一个导出入口只在工作区能解析，它不算实现。

## 对等依赖所有权

contracts、http-client、admin-web 等均为同一 `@xcss/web` 包的内部模块，通过公开子路径自引用。
它们没有独立 peer、版本或发行归档。外部 React/Vite 依赖保留精确 peer，消费者在自己的锁文件中固定。
源码与发布清单均不得含内部 `workspace:`、`file:` 或独立 `@xcss/*` 包依赖。

## 发行资产怎样生成

```text
工作树完全干净 + tag精确指向HEAD
├─ package_release -> 1个 xcss-web-1.0.2.tgz
├─ deterministic tool bundle
├─ state-contract.json
├─ hash(state contract) -> release-identity.json
├─ build-inventory.json
├─ SHA256SUMS
└─ release-tree.json（在artifacts树外）
```

xcss无runtime状态，所以状态合同的schema为null，资源/锁/外部要求/companion为空。identity target
固定为 `x86_64-unknown-linux-gnu`。xcss 的 Rust crate 和 npm 构建进程均受同一服务端平台边界约束。

## 确定性工具归档

release-tool tar只包含固定allowlist文件，source必须普通单链接且不超过上限。tar member固定mtime=0、
uid/gid=0、owner/group=root、mode和排序；gzip header也固定。相同source应生成相同字节，便于独立复核。

## 发行目录清单与 SHA256SUMS

`SHA256SUMS`覆盖所有其他artifact但不覆盖自己。release-tree描述artifacts目录中每个文件的path、mode、size、
SHA，并要求没有额外文件。清单不描述自己，避免递归hash。

verifier拒绝absolute/parent/backslash/NUL path、link/special file、超限树和TOCTOU替换。产品仍要在此后执行
产品specific目录、权限、ELF、binary self-binding和总量规则。

## 回退策略

若新xcss版本有问题，不修改已发布版本。消费者可以在源码依赖层回退到此前不可变版本并重建产品，
同时开发新的修复版本。运行时不同时加载两代xcss做兼容；软件包版本不能代替持久状态身份检查。

## 发布证据

保留：完整commit、annotated 标签、workflow run、工具链、两个lock hash、软件包归档 hash、发行身份、
状态合同、inventory、release-tree、消费者最终commit和验证结果。不要把npm/GitHub 令牌、产品Secret、
真实数据库或用户信息放入证据。
