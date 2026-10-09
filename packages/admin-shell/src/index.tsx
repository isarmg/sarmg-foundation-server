import { t } from "@xcss/admin-ui/i18n";
import { getLocale, languageLabel, switchLanguage, validationMessage } from "@xcss/admin-ui/i18n";
import {
  Component, useCallback, useEffect, useId, useRef, useState,
  type FormEvent, type ReactNode,
} from "react";
import {
  Button, ErrorState, FormField, IconButton, PageHeader, TextField, Toast,
} from "@xcss/admin-ui";
import {
  createAdministratorApiClient, type AdministratorApiClient,
} from "@xcss/admin-web";
import { useAdministratorSession } from "@xcss/admin-web/react";
import { prepareApplicationFonts } from "@xcss/web-fonts";

import { WorkspaceContext, HeaderActionsContext, HeaderNavigationContext, WorkspaceIcon } from "./workspace.js";
import { resolveWorkspaceConfig, type WorkspaceConfig } from "./workspace-config.js";
export { HeaderActions, HeaderNavigation, InstanceHeaderActions, InstancePageNavigation, InstanceNameField, WorkspaceIcon } from "./workspace.js";
export type { InstancePage } from "./workspace.js";
export { DEFAULT_WORKSPACE_CONFIG, resolveWorkspaceConfig, validInstanceName } from "./workspace-config.js";
export type { WorkspaceConfig } from "./workspace-config.js";

import { AccountSettings } from "./account.js";
export { AccountSettings, AccountPage } from "./account.js";
export { ThemeToggle, LoginControls } from "./theme.js";
import { ThemeToggle, LoginControls } from "./theme.js";
import { Context } from "./context.js";
export { useAdminApplication } from "./context.js";

export type ProductIdentity = { name: string };
export type NavigationItem = { label: string; href: string };
export type AdminApplicationOptions = {
  product: ProductIdentity;
  client?: AdministratorApiClient;
  navigation: readonly NavigationItem[];
  /** Local route selected immediately after a successful administrator login. */
  loginLandingHref?: string;
  routes: ReactNode;
  workspace?: Partial<WorkspaceConfig>;
};
const TOAST_AUTO_DISMISS_MS = 5_000;

/** Only an opaque validated identifier is rendered; never error.message or stack. */
export function errorRequestId(error: unknown): string | undefined {
  try {
    if (error && typeof error === "object" && "requestId" in error
      && typeof error.requestId === "string" && /^[A-Za-z0-9._:-]{1,128}$/.test(error.requestId)) return error.requestId;
  } catch { /* hostile/unexpected error objects are not public diagnostics */ }
  return undefined;
}

export class ApplicationErrorBoundary extends Component<{ children: ReactNode; resetKey?: string }, { failed: boolean; requestId?: string }> {
  state: { failed: boolean; requestId?: string } = { failed: false };
  static getDerivedStateFromError(error: unknown) {
    return { failed: true, requestId: errorRequestId(error) };
  }
  componentDidUpdate(previous: { resetKey?: string }) {
    if (this.state.failed && previous.resetKey !== this.props.resetKey) this.setState({ failed: false, requestId: undefined });
  }
  render() {
    return this.state.failed
      ? <ErrorState requestId={this.state.requestId} onRetry={() => this.setState({ failed: false, requestId: undefined })}>
          {t("无法显示此页面。", "This page could not be displayed.")}</ErrorState>
      : this.props.children;
  }
}

export function createXcssAdminApplication(options: AdminApplicationOptions) {
  if (!options.product.name.trim()) throw new TypeError("Product identity is required");
  const seen = new Set<string>();
  for (const item of options.navigation) {
    if (!item.label.trim() || !/^(?:\/(?!\/)|#)/.test(item.href) || /[\\\u0000-\u0020\u007f]/.test(item.href) || seen.has(item.href)) {
      throw new TypeError("Navigation requires unique local links and labels");
    }
    seen.add(item.href);
  }
  if (options.loginLandingHref !== undefined
    && (!/^(?:\/(?!\/)|#)/.test(options.loginLandingHref)
      || /[\\\u0000-\u0020\u007f]/.test(options.loginLandingHref))) {
    throw new TypeError("Login landing route must be a local link");
  }
  options = { ...options, workspace: resolveWorkspaceConfig(options.workspace) };
  const client = options.client ?? createAdministratorApiClient();
  return function XcssAdminApplication() {
    return <ApplicationErrorBoundary><AdminShell options={options} client={client} /></ApplicationErrorBoundary>;
  };
}

function AdminShell({ options, client }: { options: AdminApplicationOptions; client: AdministratorApiClient }) {
  const session = useAdministratorSession(client);
  const workspace = resolveWorkspaceConfig(options.workspace);
  const fontState = useApplicationFontsReady(workspace.fontFamily);
  const [accountUpdated, setAccountUpdated] = useState(false);
  const [logoutPending, setLogoutPending] = useState(false);
  const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const sequence = useRef(0);
  const toastTimers = useRef(new Map<number, number>());
  const [headerActions, setHeaderActions] = useState<HTMLDivElement | null>(null);
  const [headerNavigation, setHeaderNavigation] = useState<HTMLDivElement | null>(null);
  const [loginLandingPending, setLoginLandingPending] = useState(false);
  const activeFontFamily = workspace.fontFamily;
  useEffect(() => {
    if (session.phase === "authenticated" && loginLandingPending) {
      const landing = new URL(options.loginLandingHref!, window.location.href);
      if (new URL(window.location.href).searchParams.has("lang")) landing.searchParams.set("lang", getLocale());
      window.history.replaceState(null, "", landing.pathname + landing.search + landing.hash);
      setLocation(window.location.pathname + window.location.hash);
      setLoginLandingPending(false);
    }
  }, [session.phase, loginLandingPending, options.loginLandingHref]);
  useEffect(() => {
    const root = document.documentElement;
    const previous = { appearance: root.dataset.xcssAppearance, selection: root.dataset.xcssSelection, font: root.style.getPropertyValue("--xcss-font-ui"), mono: root.style.getPropertyValue("--xcss-font-mono") };
    root.dataset.xcssAppearance = workspace.appearance;
    root.dataset.xcssSelection = workspace.selection;
    root.style.setProperty("--xcss-font-ui", activeFontFamily);
    if (workspace.fontFamily.includes("Sarmg Maple")) root.style.setProperty("--xcss-font-mono", activeFontFamily);
    return () => {
      if (previous.appearance === undefined) delete root.dataset.xcssAppearance; else root.dataset.xcssAppearance = previous.appearance;
      if (previous.selection === undefined) delete root.dataset.xcssSelection; else root.dataset.xcssSelection = previous.selection;
      if (previous.mono) root.style.setProperty("--xcss-font-mono", previous.mono); else root.style.removeProperty("--xcss-font-mono");
      if (previous.font) root.style.setProperty("--xcss-font-ui", previous.font); else root.style.removeProperty("--xcss-font-ui");
    };
  }, [workspace.appearance, workspace.selection, workspace.fontFamily, activeFontFamily]);
  const dismissToast = useCallback((id: number) => {
    const timer = toastTimers.current.get(id);
    if (timer !== undefined) window.clearTimeout(timer);
    toastTimers.current.delete(id);
    setToasts(current => current.filter(item => item.id !== id));
  }, []);
  useEffect(() => () => {
    for (const timer of toastTimers.current.values()) window.clearTimeout(timer);
    toastTimers.current.clear();
  }, []);
  const notify = useCallback((message: string) => {
    const id = ++sequence.current;
    setToasts(current => [...current.slice(-4), { id, message: message.slice(0, 512) }]);
    toastTimers.current.set(id, window.setTimeout(() => {
      toastTimers.current.delete(id);
      setToasts(current => current.filter(item => item.id !== id));
    }, TOAST_AUTO_DISMISS_MS));
  }, []);
  const [location, setLocation] = useState(() => typeof window === "undefined" ? "" : window.location.pathname + window.location.hash);
  useEffect(() => {
    const changed = () => setLocation(window.location.pathname + window.location.hash);
    window.addEventListener("popstate", changed);
    window.addEventListener("hashchange", changed);
    return () => { window.removeEventListener("popstate", changed); window.removeEventListener("hashchange", changed); };
  }, []);
  useEffect(() => {
    if (session.phase !== "authenticated") {
      for (const timer of toastTimers.current.values()) window.clearTimeout(timer);
      toastTimers.current.clear();
      setToasts([]);
    }
    if (session.phase === "authenticated" && accountUpdated) setAccountUpdated(false);
  }, [session.phase, accountUpdated]);
  const identity = <div className="xcss-product-identity"><strong>{options.product.name}</strong></div>;
  if (session.phase === "loading" || fontState === "loading"
    || (session.phase === "authenticated" && loginLandingPending)) return <ApplicationBootScreen />;
  if (session.phase !== "authenticated") {
    return <div className="xcss-auth-shell" style={{ "--xcss-header-icon-size": workspace.headerIconSize } as import("react").CSSProperties}><div className="xcss-auth-language" style={{ position: "absolute", insetBlockStart: "1rem", insetInlineEnd: "1rem" }}><LoginControls><LanguageToggle /></LoginControls></div><div className="xcss-auth-content">
      {accountUpdated && <p role="status">{t("账号已更新，请使用新账号信息登录。", "Account updated. Sign in with your updated credentials.")}</p>}
      {session.phase === "anonymous_logout_unconfirmed" && <ErrorState requestId={errorRequestId(session.error)} onRetry={() => void session.logout().catch(() => {})} retryLabel={t("重试退出", "Retry sign out")}>
        {t("本地已退出，但无法确认服务器会话已注销。请重试退出以确认注销。", "Signed out locally, but the server session could not be confirmed as revoked. Retry sign out to confirm revocation.")}
      </ErrorState>}
      <div className="xcss-auth-card">{identity}
      {session.phase === "error" ? <ErrorState requestId={errorRequestId(session.error)} onRetry={() => void session.restore()}>
          {t("无法恢复管理员会话。", "Unable to restore administrator session.")}</ErrorState>
        : <LoginPage login={async (username, password) => {
          setLoginLandingPending(options.loginLandingHref !== undefined);
          try { await session.login(username, password); }
          catch (error) { setLoginLandingPending(false); throw error; }
        }} />}
    </div></div></div>;
  }
  async function logout() {
    setLogoutPending(true);
    try { await session.logout(); } catch { /* the anonymous state renders the warning */ }
    finally { setLogoutPending(false); }
  }
  return <Context.Provider value={{ client, session: session.session, notify, accountUpdated: () => setAccountUpdated(true) }}>
    <div className="xcss-admin-shell" style={{ "--xcss-header-icon-size": workspace.headerIconSize } as import("react").CSSProperties}>
      <a className="xcss-skip-link" href="#xcss-main-content" onClick={event => {
        event.preventDefault(); document.getElementById("xcss-main-content")?.focus();
      }}>{t("跳至正文", "Skip to content")}</a>
      <PageHeader><div className="xcss-header-navigation-slot"><div className="xcss-header-brand-navigation">{identity}<div ref={setHeaderNavigation} style={{ display: "contents" }}>
        {options.navigation.length > 0 && <nav className="xcss-header-navigation" aria-label={t("产品导航", "Product navigation")}>
          {options.navigation.map(item => <a key={item.href} href={item.href}
            aria-current={(item.href.startsWith("#") ? location.endsWith(item.href) : location === item.href) ? "page" : undefined}>{item.label}</a>)}
        </nav>}
      </div></div></div><div className="xcss-header-actions" role="group" aria-label={t("全局操作", "Global actions")}>
        <div ref={setHeaderActions} style={{ display: "contents" }} /><LanguageToggle /><ThemeToggle />
        <IconButton disabled={logoutPending} aria-label={logoutPending ? t("正在退出…", "Signing out…") : t("退出", "Sign out")} title={logoutPending ? t("正在退出…", "Signing out…") : t("退出", "Sign out")} onClick={() => void logout()}>{workspace.headerControls === "icons" ? <WorkspaceIcon name="logout" /> : t("退出", "Sign out")}</IconButton>
        <AccountSettings client={client} username={session.session.username} onUpdated={() => setAccountUpdated(true)} />
      </div></PageHeader>
      {toasts.length > 0 && <div className="xcss-toast-stack" role="region" aria-label={t("通知", "Notifications")}>{toasts.map(toast =>
        <Toast key={toast.id}><span>{toast.message}</span>
          <IconButton aria-label={t("关闭通知", "Dismiss notification")} onClick={() => dismissToast(toast.id)}>×</IconButton>
        </Toast>)}</div>}
      <div className="xcss-shell-layout xcss-shell-layout--full"><main id="xcss-main-content" className="xcss-shell-main" tabIndex={-1}>
        <ApplicationErrorBoundary resetKey={location}><WorkspaceContext.Provider value={workspace}><HeaderNavigationContext.Provider value={headerNavigation}><HeaderActionsContext.Provider value={headerActions}>{options.routes}</HeaderActionsContext.Provider></HeaderNavigationContext.Provider></WorkspaceContext.Provider></ApplicationErrorBoundary>
      </main></div>
    </div>
  </Context.Provider>;
}

type ApplicationFontState = "loading" | "ready";

/** The page stays blank until all application font faces have been decoded. */
function useApplicationFontsReady(fontFamily: string): ApplicationFontState {
  const usesMaple = fontFamily.includes("Sarmg Maple");
  const [state, setState] = useState<ApplicationFontState>(() =>
    typeof document === "undefined" || !usesMaple ? "ready" : "loading");
  useEffect(() => {
    if (typeof document === "undefined" || !usesMaple) {
      setState("ready");
      return;
    }
    let active = true;
    setState("loading");
    void prepareApplicationFonts().then(() => {
      if (active) setState("ready");
    }, error => { console.error("Application font preparation failed", error); });
    return () => { active = false; };
  }, [usesMaple]);
  return state;
}

function ApplicationBootScreen() {
  return <div className="xcss-application-boot" role="status" aria-busy="true"
    aria-label={t("正在准备应用…", "Preparing application…")} />;
}

export function LoginPage({ login }: { login: (username: string, password: string) => Promise<void> }) {
  const [failure, setFailure] = useState<{ message: string; requestId?: string; field?: string } | null>(null);
  const errorId = useId();
  const [pending, setPending] = useState(false);
  const submitting = useRef(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current) return;
    const form = event.currentTarget;
    for (const name of ["username", "password"]) {
      const input = form.elements.namedItem(name);
      if (input instanceof HTMLInputElement && !input.validity.valid) {
        const message = input.validity.valueMissing
          ? name === "username" ? t("请输入用户名。", "Enter your username.") : t("请输入密码。", "Enter your password.")
          : validationMessage(input);
        setFailure({ message, field: name });
        input.focus();
        return;
      }
    }
    const data = new FormData(form);
    submitting.current = true; setPending(true); setFailure(null);
    try { await login(String(data.get("username") ?? ""), String(data.get("password") ?? "")); }
    catch (error) {
      setFailure({ message: t("登录失败，请检查用户名和密码后重试。", "Sign in failed. Check your credentials and try again."), requestId: errorRequestId(error) });
      const password = form.elements.namedItem("password");
      if (password instanceof HTMLInputElement) { password.value = ""; password.focus(); }
    } finally { submitting.current = false; setPending(false); }
  }
  return <form noValidate onInvalid={event => event.preventDefault()} onInput={() => setFailure(null)} onSubmit={event => void submit(event)} aria-busy={pending}>
    <h1>{t("管理员登录", "Administrator sign in")}</h1>
    <FormField label={t("用户名", "Username")}><TextField name="username" autoComplete="username" required maxLength={64} readOnly={pending} aria-invalid={failure?.field === "username" || undefined} aria-describedby={failure ? errorId : undefined} /></FormField>
    <FormField label={t("密码", "Password")}><TextField name="password" type="password" autoComplete="current-password" required maxLength={1024} readOnly={pending} aria-invalid={failure?.field === "password" || undefined} aria-describedby={failure ? errorId : undefined} /></FormField>
    {failure && <ErrorState requestId={failure.requestId}><span id={errorId}>{failure.message}</span></ErrorState>}
    <Button type="submit" disabled={pending}>{pending ? t("正在登录…", "Signing in…") : t("登录", "Sign in")}</Button>
  </form>;
}

function LanguageToggle() {
  return <IconButton aria-label={languageLabel()} title={languageLabel()} onClick={switchLanguage}>
    <svg aria-hidden="true" width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d="M3 5h12M9 3v2M6 5c0 5 4 9 8 11M13 5c0 5-4 9-9 12m10 4 4-10 4 10m-6.5-4h5" /></svg>
  </IconButton>;
}
