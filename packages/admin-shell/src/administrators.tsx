import { t, getLocale } from "@sarmg/admin-ui/i18n";
import { useEffect, useMemo, useRef, useState, type FormEvent } from "react";
import { createAdministratorManagementClient, type AdministratorSummary } from "@sarmg/admin-web";
import { Button, ConfirmDangerDialog, Dialog, EmptyState, ErrorState, FormField, LoadingState, PageHeader, StatusBadge, Table, TextField } from "@sarmg/admin-ui";
import { errorRequestId, useAdminApplication } from "./index.js";

type Editor = { kind: "create" } | { kind: "password" | "disable"; administrator: AdministratorSummary };
const LIMIT = 50;

/** Mount only in a persistent-administrator product's business routes. */
export function AdministratorsPanel() {
  const { client, session, notify } = useAdminApplication();
  const management = useMemo(() => createAdministratorManagementClient(client), [client]);
  const [records, setRecords] = useState<AdministratorSummary[] | null>(null);
  const [failure, setFailure] = useState<{ requestId?: string } | null>(null);
  const [generation, setGeneration] = useState(0);
  const [offset, setOffset] = useState(0);
  const [editor, setEditor] = useState<Editor | null>(null);
  const [pending, setPending] = useState(false);
  const submitting = useRef(false);
  const [mutationFailure, setMutationFailure] = useState<{ requestId?: string } | null>(null);
  useEffect(() => {
    const controller = new AbortController(); setRecords(null); setFailure(null);
    void management.list(LIMIT, offset, controller.signal)
      .then(value => { if (!controller.signal.aborted) setRecords(value); })
      .catch(error => { if (!controller.signal.aborted) setFailure({ requestId: errorRequestId(error) }); });
    return () => controller.abort();
  }, [management, offset, generation]);
  const open = (value: Editor) => { setMutationFailure(null); setEditor(value); };
  const close = () => { if (!submitting.current) { setEditor(null); setMutationFailure(null); } };
  async function mutate(operation: () => Promise<void>, selfRevocation: boolean, form?: HTMLFormElement) {
    if (submitting.current) return;
    submitting.current = true; setPending(true); setMutationFailure(null);
    try {
      await operation(); setEditor(null); setGeneration(value => value + 1);
      notify(selfRevocation ? t("管理员已更新，请重新登录。", "Administrator updated. Sign in again.") : t("管理员已更新。", "Administrator updated."));
      if (selfRevocation) { await client.restore().catch(() => undefined); }
    } catch (error) {
      setMutationFailure({ requestId: errorRequestId(error) });
      const password = form?.elements.namedItem("password");
      if (password instanceof HTMLInputElement) { password.value = ""; password.focus(); }
    } finally { submitting.current = false; setPending(false); }
  }
  function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!editor || editor.kind === "disable") return;
    const form = event.currentTarget;
    const data = new FormData(form);
    const password = String(data.get("password") ?? "");
    if (editor.kind === "create") {
      void mutate(() => management.create(String(data.get("username") ?? ""), password), false, form);
    } else {
      const administrator = editor.administrator;
      void mutate(() => management.setPassword(administrator.administrator_id, password), administrator.administrator_id === session.user_id, form);
    }
  }
  return <section aria-label={t("管理员管理", "Administrator management")}>
    <PageHeader><h2>{t("管理员", "Administrators")}</h2><Button onClick={() => open({ kind: "create" })}>{t("创建管理员", "Create administrator")}</Button></PageHeader>
    <p>{t("所有账户均具有管理员权限。修改密码或停用账户会撤销该账户的所有会话。不能停用最后一位有效管理员。", "All accounts have administrator access. Password changes and disabling revoke every session for that account. The final active administrator cannot be disabled.")}</p>
    {failure ? <ErrorState requestId={failure.requestId} onRetry={() => setGeneration(value => value + 1)}>{t("无法加载管理员列表。", "Administrators could not be loaded.")}</ErrorState>
      : records === null ? <LoadingState>{t("正在加载管理员…", "Loading administrators…")}</LoadingState>
      : records.length === 0 ? <EmptyState>{t("本页暂无管理员。", "No administrators on this page.")}</EmptyState>
      : <Table aria-label={t("管理员", "Administrators")}><thead><tr><th scope="col">{t("用户名", "Username")}</th><th scope="col">{t("状态", "Status")}</th><th scope="col">{t("最近登录", "Last sign in")}</th><th scope="col">{t("操作", "Actions")}</th></tr></thead>
        <tbody>{records.map(record => <tr key={record.administrator_id}>
          <th scope="row">{record.username}{record.administrator_id === session.user_id ? t("（本人）", " (you)") : ""}</th>
          <td><StatusBadge status={record.active ? t("有效", "Active") : t("已停用", "Disabled")} /></td>
          <td>{record.last_login_at_micros === null ? t("从未登录", "Never") : new Date(record.last_login_at_micros / 1000).toLocaleString(getLocale())}</td>
          <td><div className="sarmg-actions"><Button disabled={!record.active} aria-label={t("修改 {0} 的密码", "Change password for {0}", [record.username])} onClick={() => open({ kind: "password", administrator: record })}>{t("修改密码", "Change password")}</Button>
            <Button disabled={!record.active} aria-label={t("停用 {0}", "Disable {0}", [record.username])} onClick={() => open({ kind: "disable", administrator: record })}>{t("停用", "Disable")}</Button></div></td>
        </tr>)}</tbody></Table>}
    <nav className="sarmg-actions" aria-label={t("管理员分页", "Administrator pages")}>
      <Button disabled={offset === 0 || records === null} onClick={() => setOffset(value => Math.max(0, value - LIMIT))}>{t("上一页管理员", "Previous administrators")}</Button>
      <span>{t("第 {0} 页", "Page {0}", [offset / LIMIT + 1])}</span>
      <Button disabled={records === null || records.length < LIMIT} onClick={() => setOffset(value => value + LIMIT)}>{t("下一页管理员", "Next administrators")}</Button>
    </nav>
    {editor?.kind === "disable" ? <ConfirmDangerDialog title={t("停用 {0}？", "Disable {0}?", [editor.administrator.username])} description={t("此操作会撤销该账户的所有会话。停用的账户不能登录。", "This revokes every session for this account. Disabled accounts cannot sign in.")} pending={pending} onClose={close}
        onConfirm={() => { const administrator = editor.administrator; void mutate(() => management.disable(administrator.administrator_id), administrator.administrator_id === session.user_id); }}>
      {mutationFailure && <ErrorState requestId={mutationFailure.requestId}>{t("无法停用管理员，必须保留至少一位有效管理员。", "Administrator could not be disabled. The final active administrator must remain enabled.")}</ErrorState>}
    </ConfirmDangerDialog> : editor && <Dialog title={editor.kind === "create" ? t("创建管理员", "Create administrator") : t("修改 {0} 的密码", "Change password for {0}", [editor.administrator.username])} onClose={close}>
      <form onSubmit={submit} aria-busy={pending}>
        {mutationFailure && <ErrorState requestId={mutationFailure.requestId}>{t("无法更新管理员，请检查输入后重试。", "Administrator could not be updated. Check the input and try again.")}</ErrorState>}
        {editor.kind === "create" && <FormField label={t("用户名", "Username")}><TextField name="username" required minLength={3} maxLength={64} autoComplete="off" readOnly={pending} /></FormField>}
        <FormField label={t("新密码", "New password")}><TextField name="password" type="password" required minLength={12} maxLength={1024} autoComplete="new-password" readOnly={pending} /></FormField>
        <div className="sarmg-actions"><Button disabled={pending} onClick={close}>{t("取消", "Cancel")}</Button><Button type="submit" disabled={pending}>{pending ? t("正在保存…", "Saving…") : t("保存管理员", "Save administrator")}</Button></div>
      </form>
    </Dialog>}
  </section>;
}
