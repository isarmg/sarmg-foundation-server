# 可编辑日期范围

`@xcss/admin-ui/date-range` 提供 `DateRangeField`、`CalendarDateRange` 和公历日期校验；样式随 `@xcss/admin-ui/styles.css` 加载，也可单独导入 `@xcss/admin-ui/date-range.css`。

控件显示 `年/月/日-年/月/日`，六个数字独立编辑，使用虚线下划线，聚焦时加粗下划线，不显示方框或常驻提示。草稿不会发起查询，任意数字内按回车才调用 `onApply`，输出规范的 `YYYY-MM-DD` 起止日期。空值、非数字、非法月份、闰年错误和起止倒置阻止应用；错误只影响数字，斜线和范围分隔符保持正常颜色。错误有文字提示和 `aria-invalid`，输入法确认键不触发应用。`onValidityChange` 可让业务刷新按钮避免在非法草稿期间发起查询，`serverInvalid` 可呈现服务器额外的日期范围限制。

xcss 不拥有日志 API、服务器时区或业务权限。消费者必须在后端独立验证日期、完整起止参数和顺序，以服务器时区分别求开始日午夜与结束日次日午夜，包含两个端点整天，避免固定 24 小时的夏令时错误。分页游标应绑定完整范围及权限域，改范围必须回到首页。

当前四个产品仍固定不可变的 xcss 0.11.2 包。发布包含此控件的新版前，产品使用本仓库生成的受校验源码快照，只有 i18n 导入路径改为固定包入口，没有修改已发布依赖或跨仓库运行时引用。

```sh
node scripts/sync-date-range.mjs --write # 更新四个产品快照
node scripts/sync-date-range.mjs         # 校验与上游完全一致
```
