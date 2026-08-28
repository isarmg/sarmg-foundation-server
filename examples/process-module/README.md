# Process plugin package example

`manifest.json` 可直接复制后改名。Builder 负责校验 manifest、替换 `backend/README.md` 为目标
平台可执行文件，并把整个目录作为当前 Union 发行的一部分。Core 只发现 Builder 已打包的目录，
不下载公网插件。

固定顶层布局：

```text
manifest.json
backend/<executable>
frontend/entry.js
frontend/styles.css
permissions/definitions.json
config/schema.json
migrations/*.sql
version/metadata.json
```

前端入口不得打包或运行时导入第二份 React/ReactDOM。Shell 将唯一实例作为 `hostSdk.react`
传给 `activate(hostSdk)`；返回的 `components` key 必须覆盖 manifest 的组件白名单。
