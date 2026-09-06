import { t } from "@sarmg/admin-ui/i18n";
import { languageLabel, switchLanguage } from "@sarmg/admin-ui/i18n";
import {
  Component, createContext, useCallback, useContext, useEffect, useRef, useState,
  type FormEvent, type ReactNode,
} from "react";
import {
  Button, ErrorState, FormField, IconButton, LoadingState, PageHeader, TextField, Toast,
} from "@sarmg/admin-ui";
import {
  createAdministratorApiClient, type AdministratorApiClient,
} from "@sarmg/admin-web";
import { useAdministratorSession, type AdministratorSessionController } from "@sarmg/admin-web/react";

import { WorkspaceContext, HeaderActionsContext, HeaderNavigationContext, WorkspaceIcon } from "./workspace.js";
import { resolveWorkspaceConfig, type WorkspaceConfig } from "./workspace-config.js";
export { HeaderActions, HeaderNavigation, InstanceHeaderActions, InstanceWorkspace, InstanceNameField, WorkspaceIcon } from "./workspace.js";
export { DEFAULT_WORKSPACE_CONFIG, resolveWorkspaceConfig, validInstanceName } from "./workspace-config.js";
export type { WorkspaceConfig } from "./workspace-config.js";

export { AdministratorsPanel } from "./administrators.js";

export type ProductIdentity = { name: string };
export type NavigationItem = { label: string; href: string };
export type AdminApplicationOptions = {
  product: ProductIdentity;
  client?: AdministratorApiClient;
  navigation: readonly NavigationItem[];
  routes: ReactNode;
  workspace?: Partial<WorkspaceConfig>;
};
type ApplicationContext = {
  client: AdministratorApiClient;
  session: NonNullable<AdministratorSessionController["session"]>;
  notify(message: string): void;
};
const Context = createContext<ApplicationContext | null>(null);
export function useAdminApplication(): ApplicationContext {
  const context = useContext(Context);
  if (!context) throw new Error("Product routes must be inside the administrator application");
  return context;
}

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

export function createSarmgAdminApplication(options: AdminApplicationOptions) {
  if (!options.product.name.trim()) throw new TypeError("Product identity is required");
  const seen = new Set<string>();
  for (const item of options.navigation) {
    if (!item.label.trim() || !/^(?:\/(?!\/)|#)/.test(item.href) || /[\\\u0000-\u0020\u007f]/.test(item.href) || seen.has(item.href)) {
      throw new TypeError("Navigation requires unique local links and labels");
    }
    seen.add(item.href);
  }
  options = { ...options, workspace: resolveWorkspaceConfig(options.workspace) };
  const client = options.client ?? createAdministratorApiClient();
  return function SarmgAdminApplication() {
    return <ApplicationErrorBoundary><AdminShell options={options} client={client} /></ApplicationErrorBoundary>;
  };
}

function AdminShell({ options, client }: { options: AdminApplicationOptions; client: AdministratorApiClient }) {
  const session = useAdministratorSession(client);
  const [logoutPending, setLogoutPending] = useState(false);
  const [logoutError, setLogoutError] = useState<unknown>(null);
  const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const sequence = useRef(0);
  const [headerActions, setHeaderActions] = useState<HTMLDivElement | null>(null);
  const [headerNavigation, setHeaderNavigation] = useState<HTMLDivElement | null>(null);
  const workspace = resolveWorkspaceConfig(options.workspace);
  useEffect(() => {
    const root = document.documentElement;
    const previous = { appearance: root.dataset.sarmgAppearance, selection: root.dataset.sarmgSelection, font: root.style.getPropertyValue("--sarmg-font-ui") };
    root.dataset.sarmgAppearance = workspace.appearance;
    root.dataset.sarmgSelection = workspace.selection;
    root.style.setProperty("--sarmg-font-ui", workspace.fontFamily);
    return () => {
      if (previous.appearance === undefined) delete root.dataset.sarmgAppearance; else root.dataset.sarmgAppearance = previous.appearance;
      if (previous.selection === undefined) delete root.dataset.sarmgSelection; else root.dataset.sarmgSelection = previous.selection;
      if (previous.font) root.style.setProperty("--sarmg-font-ui", previous.font); else root.style.removeProperty("--sarmg-font-ui");
    };
  }, [workspace.appearance, workspace.selection, workspace.fontFamily]);
  const notify = useCallback((message: string) => {
    const id = ++sequence.current;
    setToasts(current => [...current.slice(-4), { id, message: message.slice(0, 512) }]);
  }, []);
  const [location, setLocation] = useState(() => typeof window === "undefined" ? "" : window.location.pathname + window.location.hash);
  useEffect(() => {
    const changed = () => setLocation(window.location.pathname + window.location.hash);
    window.addEventListener("popstate", changed);
    window.addEventListener("hashchange", changed);
    return () => { window.removeEventListener("popstate", changed); window.removeEventListener("hashchange", changed); };
  }, []);
  useEffect(() => {
    if (session.phase !== "authenticated") { setToasts([]); setLogoutError(null); }
  }, [session.phase]);
  const identity = <div className="sarmg-product-identity"><strong>{options.product.name}</strong></div>;
  if (session.phase !== "authenticated") {
    return <div className="sarmg-auth-shell"><div className="sarmg-auth-language" style={{ position: "absolute", insetBlockStart: "1rem", insetInlineEnd: "1rem" }}><LanguageToggle /></div><div className="sarmg-auth-card">{identity}
      {session.phase === "loading" ? <LoadingState>{t("正在恢复管理员会话…", "Restoring administrator session…")}</LoadingState>
        : session.phase === "error" ? <ErrorState requestId={errorRequestId(session.error)} onRetry={() => void session.restore()}>
          {t("无法恢复管理员会话。", "Unable to restore administrator session.")}</ErrorState>
        : <LoginPage login={session.login} />}
    </div></div>;
  }
  async function logout() {
    setLogoutPending(true); setLogoutError(null);
    try { await session.logout(); } catch (error) { setLogoutError(error); }
    finally { setLogoutPending(false); }
  }
  return <Context.Provider value={{ client, session: session.session, notify }}>
    <div className="sarmg-admin-shell" style={{ "--sarmg-header-icon-size": workspace.headerIconSize } as import("react").CSSProperties}>
      <a className="sarmg-skip-link" href="#sarmg-main-content" onClick={event => {
        event.preventDefault(); document.getElementById("sarmg-main-content")?.focus();
      }}>{t("跳至正文", "Skip to content")}</a>
      <PageHeader><div className="sarmg-header-navigation-slot"><div className="sarmg-header-brand-navigation">{identity}<div ref={setHeaderNavigation} style={{ display: "contents" }}>
        {options.navigation.length > 0 && <nav className="sarmg-header-navigation" aria-label={t("产品导航", "Product navigation")}>
          {options.navigation.map(item => <a key={item.href} href={item.href}
            aria-current={(item.href.startsWith("#") ? location.endsWith(item.href) : location === item.href) ? "page" : undefined}>{item.label}</a>)}
        </nav>}
      </div></div></div><div className="sarmg-header-actions" role="group" aria-label={t("全局操作", "Global actions")}>
        <div ref={setHeaderActions} style={{ display: "contents" }} /><LanguageToggle /><ThemeToggle />
        <IconButton disabled={logoutPending} aria-label={logoutPending ? t("正在退出…", "Signing out…") : t("退出", "Sign out")} title={logoutPending ? t("正在退出…", "Signing out…") : t("退出", "Sign out")} onClick={() => void logout()}>{workspace.headerControls === "icons" ? <WorkspaceIcon name="logout" /> : t("退出", "Sign out")}</IconButton>
      </div></PageHeader>
      {toasts.length > 0 && <div className="sarmg-toast-stack" role="region" aria-label={t("通知", "Notifications")}>{toasts.map(toast =>
        <Toast key={toast.id}><span>{toast.message}</span>
          <IconButton aria-label={t("关闭通知", "Dismiss notification")} onClick={() => setToasts(current => current.filter(item => item.id !== toast.id))}>×</IconButton>
        </Toast>)}</div>}
      <div className="sarmg-shell-layout sarmg-shell-layout--full"><main id="sarmg-main-content" className="sarmg-shell-main" tabIndex={-1}>
        {logoutError !== null && <ErrorState requestId={errorRequestId(logoutError)}>{t("无法确认退出结果，请重试。", "Sign out could not be confirmed. Try again.")}</ErrorState>}
        <ApplicationErrorBoundary resetKey={location}><WorkspaceContext.Provider value={workspace}><HeaderNavigationContext.Provider value={headerNavigation}><HeaderActionsContext.Provider value={headerActions}>{options.routes}</HeaderActionsContext.Provider></HeaderNavigationContext.Provider></WorkspaceContext.Provider></ApplicationErrorBoundary>
      </main></div>
    </div>
  </Context.Provider>;
}

export function LoginPage({ login }: { login: (username: string, password: string) => Promise<void> }) {
  const [failure, setFailure] = useState<{ requestId?: string } | null>(null);
  const [pending, setPending] = useState(false);
  const submitting = useRef(false);
  async function submit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (submitting.current) return;
    const form = event.currentTarget;
    const data = new FormData(form);
    submitting.current = true; setPending(true); setFailure(null);
    try { await login(String(data.get("username") ?? ""), String(data.get("password") ?? "")); }
    catch (error) {
      setFailure({ requestId: errorRequestId(error) });
      const password = form.elements.namedItem("password");
      if (password instanceof HTMLInputElement) { password.value = ""; password.focus(); }
    } finally { submitting.current = false; setPending(false); }
  }
  return <form onSubmit={event => void submit(event)} aria-busy={pending}>
    <h1>{t("管理员登录", "Administrator sign in")}</h1>
    {failure && <ErrorState requestId={failure.requestId}>{t("登录失败，请检查用户名和密码后重试。", "Sign in failed. Check your credentials and try again.")}</ErrorState>}
    <FormField label={t("用户名", "Username")}><TextField name="username" autoComplete="username" required maxLength={64} readOnly={pending} /></FormField>
    <FormField label={t("密码", "Password")}><TextField name="password" type="password" autoComplete="current-password" required maxLength={1024} readOnly={pending} /></FormField>
    <Button type="submit" disabled={pending}>{pending ? t("正在登录…", "Signing in…") : t("登录", "Sign in")}</Button>
  </form>;
}

function LanguageToggle() {
  return <IconButton aria-label={languageLabel()} title={languageLabel()} onClick={switchLanguage}>
    <svg aria-hidden="true" width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d="M3 5h12M9 3v2M6 5c0 5 4 9 8 11M13 5c0 5-4 9-9 12m10 4 4-10 4 10m-6.5-4h5" /></svg>
  </IconButton>;
}

function ThemeToggle() {
  const [theme, setTheme] = useState(() => typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
  useEffect(() => {
    const root = document.documentElement;
    const previous = root.dataset.theme;
    root.dataset.theme = theme;
    return () => { if (previous === undefined) delete root.dataset.theme; else root.dataset.theme = previous; };
  }, [theme]);
  const label = theme === "light" ? t("切换到深色模式", "Switch to dark mode") : t("切换到浅色模式", "Switch to light mode");
  return <IconButton aria-label={label} title={label} onClick={() => setTheme(current => current === "light" ? "dark" : "light")}>
    <svg aria-hidden="true" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      {theme === "light" ? <path d="M20.8 13A9 9 0 0 1 11 3.2 9 9 0 1 0 20.8 13Z" /> : <><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M5 19l1.4-1.4M17.6 6.4 19 5" /></>}
    </svg>
  </IconButton>;
}
