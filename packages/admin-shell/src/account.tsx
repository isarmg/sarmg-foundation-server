import { isAdministratorPassword, type AdministratorApiClient } from "@xcss/admin-web";
import { Context } from "./context.js";
import { Button, ErrorState, FormField, IconButton, TextField } from "@xcss/admin-ui";
import { t, validationMessage } from "@xcss/admin-ui/i18n";
import { useContext, useId, useRef, useState, type FormEvent } from "react";

type Failure = { message: string; field?: string };

export type AccountSettingsProps = {
  client?: AdministratorApiClient; username?: string; onUpdated?(): void;
  onNavigate?(): void; href?: string;
};

/** The account menu entry has no selected state, even while its page is open. */
export function AccountSettings({ onNavigate, href = "#account" }: AccountSettingsProps = {}) {
  return <IconButton className="xcss-account-entry" aria-label={t("账号设置", "Account settings")}
    title={t("账号设置", "Account settings")} style={{ borderRadius: "50%", aspectRatio: "1" }}
    onClick={() => { if (onNavigate) onNavigate(); else window.location.hash = href; }}>
    <svg viewBox="0 0 24 24" width="1em" height="1em" fill="none" stroke="currentColor" strokeWidth="1.7" aria-hidden="true">
      <circle cx="12" cy="8" r="3.5" /><path d="M4.5 21v-2a7.5 7.5 0 0 1 15 0v2" />
    </svg>
  </IconButton>;
}

/** Ordinary route content; explicit props also support custom product shells. */
export function AccountPage({ client: explicitClient, username: explicitUsername, onUpdated }: AccountSettingsProps = {}) {
  const application = useContext(Context);
  const client = explicitClient ?? application?.client;
  const username = explicitUsername ?? application?.session.username;
  if (!client || username === undefined) throw new Error("Account page requires an authenticated administrator application");
  return <AccountForm client={client} username={username} onUpdated={onUpdated ?? application?.accountUpdated} />;
}

function AccountForm({ client, username, onUpdated }: { client: AdministratorApiClient; username: string; onUpdated?(): void }) {
  const [failure, setFailure] = useState<Failure | null>(null);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const errorId = useId();
  const helpId = useId();

  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (busy.current) return;
    const form = event.currentTarget;
    function invalid(field: string, message: string) {
      setFailure({ field, message });
      const input = form.elements.namedItem(field);
      if (input instanceof HTMLInputElement) input.focus();
    }
    for (const name of ["username", "current_password", "new_password", "confirm_password"]) {
      const input = form.elements.namedItem(name) as HTMLInputElement;
      if (!input.validity.valid) {
        invalid(name, input.validity.valueMissing
          ? name === "username" ? t("请输入用户名。", "Enter your username.") : t("请输入当前密码。", "Enter your current password.")
          : validationMessage(input));
        return;
      }
    }
    const data = new FormData(form);
    const password = String(data.get("new_password"));
    if (password && !isAdministratorPassword(password)) {
      invalid("new_password", t("新密码长度不符合要求。", "The new password length is invalid."));
      return;
    }
    if (password !== data.get("confirm_password")) {
      invalid("confirm_password", t("两次输入的新密码不一致。", "The new passwords do not match."));
      return;
    }
    busy.current = true;
    setPending(true);
    setFailure(null);
    try {
      // Foundation verifies the current password and updates the account atomically.
      await client.updateAccount({ username: String(data.get("username")), current_password: String(data.get("current_password")),
        ...(password ? { new_password: password } : {}) });
      form.reset();
      onUpdated?.();
    } catch (error) {
      const code = error && typeof error === "object" && "code" in error ? error.code : undefined;
      setFailure(code === "admin.current_password_invalid"
        ? { field: "current_password", message: t("当前密码不正确。", "The current password is incorrect.") }
        : code === "admin.conflict" ? { field: "username", message: t("用户名已被使用。", "The username is already in use.") }
        : { message: t("账号未能更新，请检查输入并重试。", "The account could not be updated. Check the input and retry.") });
      for (const name of ["current_password", "new_password", "confirm_password"]) {
        (form.elements.namedItem(name) as HTMLInputElement).value = "";
      }
      const input = form.elements.namedItem(code === "admin.conflict" ? "username" : "current_password");
      if (input instanceof HTMLInputElement) input.focus();
    } finally {
      busy.current = false;
      setPending(false);
    }
  }

  const describedBy = failure ? errorId : undefined;
  return <section className="xcss-content-stack xcss-account-page" aria-label={t("账号设置", "Account settings")}>
    <div className="xcss-content-panel">
      <h2>{t("账号设置", "Account settings")}</h2>
      <p id={helpId}>{t("新密码留空则保留现有密码；保存后需重新登录。", "Leave the new password blank to keep it. Sign in again after saving.")}</p>
      <form className="xcss-account-form" noValidate onInvalid={event => event.preventDefault()} onInput={() => setFailure(null)}
        onSubmit={event => void submit(event)} aria-busy={pending} aria-describedby={helpId}>
        <div className="xcss-account-fields">
        <FormField label={t("用户名", "Username")}><TextField name="username" defaultValue={username} autoComplete="username"
          required minLength={3} maxLength={64} readOnly={pending} aria-invalid={failure?.field === "username" || undefined} aria-describedby={describedBy} /></FormField>
        <FormField label={t("当前密码", "Current password")}><TextField name="current_password" type="password" autoComplete="current-password"
          required maxLength={1024} readOnly={pending} aria-invalid={failure?.field === "current_password" || undefined} aria-describedby={describedBy} /></FormField>
        <FormField label={t("新密码", "New password")}><TextField name="new_password" type="password" autoComplete="new-password"
          maxLength={1024} readOnly={pending} aria-invalid={failure?.field === "new_password" || undefined} aria-describedby={describedBy} /></FormField>
        <FormField label={t("确认新密码", "Confirm new password")}><TextField name="confirm_password" type="password" autoComplete="new-password"
          maxLength={1024} readOnly={pending} aria-invalid={failure?.field === "confirm_password" || undefined} aria-describedby={describedBy} /></FormField>
        </div>
        {failure && <ErrorState><span id={errorId}>{failure.message}</span></ErrorState>}
        <div className="xcss-actions"><Button type="submit" disabled={pending}>{pending ? t("保存中…", "Saving…") : t("保存", "Save")}</Button></div>
      </form>
    </div>
  </section>;
}
