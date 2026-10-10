# 服务端公共支撑与产品职责

xcss 为服务端产品提供一个 Rust crate 和一个管理 Web npm 包。Rust 能力通过 `xcss::<module>` 使用，Web 能力通过 `@xcss/web/<module>` 使用；内部模块共享版本、源码 revision、许可证与 Linux AMD64 GNU 平台边界。

## 职责判定

| 所属 | 应负责 | 验收依据 |
|---|---|---|
| xcss | Server 进程机制、管理 Server 的 Web、管理员认证、服务端 HTTP/数据库/文件原语、通用管理 UI | 公共合同、产品无关测试、单体构建、实际发行包与已验证的 Profile |
| 产品服务端 | 端点与 wire、实例/设备/硬件状态机、业务错误目录、业务页面、更严格的产品安全规则和产品发布验收 | 固定的公共版本与完整 revision、产品清单、业务测试和实际部署验证 |

进入 xcss 的能力必须产品中立、能在 xcss 内独立测试、允许产品继续加强约束，并有明确的跨产品复用场景。单产品能力先留在产品；不能因为多个产品都叫“配对”就把不同 wire、状态码和恢复流程合并成公共协议。

## 接入与来源

服务端组件使用 `server-control-plane` 或 `server-filesystem`；管理 Web 使用 `web-react-admin` 或 `web-embedded-native`。`xcss-product.toml` 声明具体组合与能力，产品固定 Rust 完整 Git revision、精确版本及唯一 Web 发行 tarball 的 integrity。

`@xcss/web/admin-ui` 的内容块提供布局、色板和无障碍展示原语。实例统计、授权码、CPU/GPU/SSD/RAM、摄像头、Sunshine 控制等内容与行为由产品定义。消费者从锁定的发布包导入样式，不在产品仓库保存公共 CSS 的另一份事实源。

公共库通过编译依赖接入，无需单独安装后台服务。服务的初始化、运行、诊断、启停和卸载由具体产品的部署流程完成。
