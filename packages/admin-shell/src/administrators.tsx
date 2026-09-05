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
      notify(selfRevocation ? "Administrator updated. Sign in again." : "Administrator updated.");
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
  return <section aria-label="Administrator management">
    <PageHeader><h2>Administrators</h2><Button onClick={() => open({ kind: "create" })}>Create administrator</Button></PageHeader>
    <p>All accounts have administrator access. Password changes and disabling revoke every session for that account. The final active administrator cannot be disabled.</p>
    {failure ? <ErrorState requestId={failure.requestId} onRetry={() => setGeneration(value => value + 1)}>Administrators could not be loaded.</ErrorState>
      : records === null ? <LoadingState>Loading administrators…</LoadingState>
      : records.length === 0 ? <EmptyState>No administrators on this page.</EmptyState>
      : <Table aria-label="Administrators"><thead><tr><th scope="col">Username</th><th scope="col">Status</th><th scope="col">Last sign in</th><th scope="col">Actions</th></tr></thead>
        <tbody>{records.map(record => <tr key={record.administrator_id}>
          <th scope="row">{record.username}{record.administrator_id === session.user_id ? " (you)" : ""}</th>
          <td><StatusBadge status={record.active ? "Active" : "Disabled"} /></td>
          <td>{record.last_login_at_micros === null ? "Never" : new Date(record.last_login_at_micros / 1000).toLocaleString()}</td>
          <td><div className="sarmg-actions"><Button disabled={!record.active} aria-label={`Change password for ${record.username}`} onClick={() => open({ kind: "password", administrator: record })}>Change password</Button>
            <Button disabled={!record.active} aria-label={`Disable ${record.username}`} onClick={() => open({ kind: "disable", administrator: record })}>Disable</Button></div></td>
        </tr>)}</tbody></Table>}
    <nav className="sarmg-actions" aria-label="Administrator pages">
      <Button disabled={offset === 0 || records === null} onClick={() => setOffset(value => Math.max(0, value - LIMIT))}>Previous administrators</Button>
      <span>Page {offset / LIMIT + 1}</span>
      <Button disabled={records === null || records.length < LIMIT} onClick={() => setOffset(value => value + LIMIT)}>Next administrators</Button>
    </nav>
    {editor?.kind === "disable" ? <ConfirmDangerDialog title={`Disable ${editor.administrator.username}?`} description="This revokes every session for this account. Disabled accounts cannot sign in." pending={pending} onClose={close}
        onConfirm={() => { const administrator = editor.administrator; void mutate(() => management.disable(administrator.administrator_id), administrator.administrator_id === session.user_id); }}>
      {mutationFailure && <ErrorState requestId={mutationFailure.requestId}>Administrator could not be disabled. The final active administrator must remain enabled.</ErrorState>}
    </ConfirmDangerDialog> : editor && <Dialog title={editor.kind === "create" ? "Create administrator" : `Change password for ${editor.administrator.username}`} onClose={close}>
      <form onSubmit={submit} aria-busy={pending}>
        {mutationFailure && <ErrorState requestId={mutationFailure.requestId}>Administrator could not be updated. Check the input and try again.</ErrorState>}
        {editor.kind === "create" && <FormField label="Username"><TextField name="username" required minLength={3} maxLength={64} autoComplete="off" readOnly={pending} /></FormField>}
        <FormField label="New password"><TextField name="password" type="password" required minLength={12} maxLength={1024} autoComplete="new-password" readOnly={pending} /></FormField>
        <div className="sarmg-actions"><Button disabled={pending} onClick={close}>Cancel</Button><Button type="submit" disabled={pending}>{pending ? "Saving…" : "Save administrator"}</Button></div>
      </form>
    </Dialog>}
  </section>;
}
