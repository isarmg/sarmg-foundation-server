# 09. 供应链、安全与维护运维

## 9.1 这里的“运维”

Foundation 无 daemon，运维对象是仓库、依赖锁、CI、registry 权限、构建与不可变 package。一次供应链
问题可能传播到多个产品，因此影响面通常大于普通工具库。

## 9.2 CI 边界

Actions 锁定完整 commit SHA、最小只读权限、job timeout，checkout 不保留 credential。发布 job 与测试
证据绑定；fork/不可信 PR 不获得 registry Secret。

## 9.3 Registry 权限

Cargo/npm 发布 Token 使用最小 scope、短期或可信发布机制，保存在平台 Secret，不进入本地文件、日志或
制品。维护者变更和 Token 轮换留审计记录。

## 9.4 依赖响应

收到漏洞公告后确认版本、feature、可达代码和所有消费者；在 Foundation 修复并发布新不可变版本，再让
每个产品更新锁、重建发行物。不能通过生产 CDN 热替换共享代码。

## 9.5 Package 内容

发布前列清单并拒绝源码树 Secret、测试数据、缓存、`.git`、node_modules、target 和不必要脚本。package
metadata、license、readme、types/exports 与 checksum 可复核。

## 9.6 监测

关注 CI 固定 action 漂移、依赖公告、registry ownership、异常发布、下载来源和消费者版本。定期从空缓存
安装并运行 package test。

## 9.7 事件处置

暂停发布，撤销 Token，保全 workflow/run/package digests，确认受影响版本与消费者，发布新的修复版本并
逐产品重建。不要覆盖或删除已发布版本来隐藏事件。

## 9.8 备份

源码 Git、tag、release metadata 和 CI 配置需可恢复；registry 不是唯一源码备份。Foundation 无生产业务
数据库，因此不要把消费者 backup 功能加入本仓。

## 9.9 只支持当前

安全修复进入当前版本与 `main`。不为另一代 package 增加运行时兼容层；需要继续运行旧产品的风险由其
独立生命周期处理。
