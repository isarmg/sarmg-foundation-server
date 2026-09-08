import { t } from "@sarmg/admin-ui/i18n";
import { Table } from "@sarmg/admin-ui";
import { useAdminApplication } from "./index.js";

/** Account information for consumers that retain a system settings page. */
export function AdministratorsPanel() {
  const { session } = useAdminApplication();
  return <section><h2>{t("管理员账号", "Administrator account")}</h2>
    <Table aria-label={t("管理员账号", "Administrator account")}><thead><tr><th scope="col">{t("账号名称", "Account name")}</th><th scope="col">{t("角色", "Role")}</th></tr></thead><tbody><tr><th scope="row">{session.username}</th><td>{t("管理员", "Administrator")}</td></tr></tbody></Table>
    <p>{t("本系统仅允许一个管理员。修改账号名称和密码，请使用右上角人物图标。", "This system has one administrator. Use the person icon at the top right to change the account name or password.")}</p>
  </section>;
}
