# P0 冻结基线

每个 TOML 固定产品 commit、版本、状态种类、真实 current Schema fingerprint，以及当时使用的 Foundation
版本/revision。`fixture` 指向 `sarmg-upgrade` 中不可变的脱敏 current-state source fixture；每套 fixture
均由测试重建 SQLite、复算 fingerprint、检查管理员/Session/业务/审计记录和外键完整性。

后续持久格式变更必须新增版本目录，并在 `sarmg-upgrade` 注册精确升级边；不得就地修改本目录记录的历史
source fixture。

本目录的 `source_commit` 和 Foundation revision 是 P0 历史证据，不随当前采用进度前移。
`consumers/repositories.toml` 与自动生成的 `consumer-matrix.json` 才记录最近已验证消费者 commit；
合规工具会分别校验两类 revision 的形式与产品集，不强迫它们相等。
