# 原生嵌入式 Web Profile

`web-embedded-native` 由 Foundation 拥有原生 ESM 管理员客户端、当前合同、设计 token、Maple 字体和构建策略。产品只保留原生业务页面与 HTTP/XHR 业务协议 Adapter，不引入 React。

`@sarmg/web-toolchain/native` 提供唯一 Vite 配置：ES 输出、保留公开入口、相对资源 URL、外置字体、无 source map、单资源 256 KiB 硬预算。默认产物为 `platform.js`、`platform.css` 及字体和许可证；产品可以从官方字体包的 `OFL.txt` 导出引用，构建时必须包含许可证。产品不能复制平台源码到自己的 vendor 目录。

产品将生成的资源作为构建输入嵌入二进制，以资源名、媒体类型和实际字节共同计算缓存身份。声明文件从产品的纯再导出入口生成，不检查压缩后的 JS，不手写第二套平台类型。

页面 HTML 仅嵌入业务元数据。登录、恢复和退出使用每个文档唯一的 `createAdministratorApiClient`；恢复完成前不得启动受保护业务 UI。会话与 CSRF 不进入 HTML、localStorage 或 sessionStorage。密码的表单策略由 `isAdministratorPassword` 判断 UTF-8 字节范围 12–1024，拒绝控制字符和孤立代理项；这不同于 wire DTO 的有界候选字符串校验。

登录操作必须有 pending 防重复提交，结束时清除密码，失败时恢复可操作状态并将焦点返回密码。显示固定安全提示及经平台验证的 Request ID，不显示任意异常正文。CSP 使用同源外置 ESM/CSS/font，不允许内联脚本或 eval。

适用产品的业务上传协议仍归产品。401 可在响应头阶段结束并取消未读正文；403 必须有有界正文和有效 Foundation ErrorEnvelope 后才能按 `auth.csrf_rejected` 处理，不得恢复产品私有认证响应头或错误码。

此能力不改变持久状态，不提供旧客户端适配或升级边。第一消费者删除自有登录 fetch、Session shape 校验和内联 CSRF 引导，改用上述平台机制。
