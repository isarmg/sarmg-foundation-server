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
          This page could not be displayed.
        </ErrorState>
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
    return <div className="sarmg-auth-shell"><div className="sarmg-auth-card">{identity}
      {session.phase === "loading" ? <LoadingState>Restoring administrator session…</LoadingState>
        : session.phase === "error" ? <ErrorState requestId={errorRequestId(session.error)} onRetry={() => void session.restore()}>
          Unable to restore administrator session.
        </ErrorState>
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
      }}>Skip to content</a>
      <PageHeader><div className="sarmg-header-navigation-slot"><div className="sarmg-header-brand-navigation">{identity}<div ref={setHeaderNavigation} style={{ display: "contents" }}>
        {options.navigation.length > 0 && <nav className="sarmg-header-navigation" aria-label="Product navigation">
          {options.navigation.map(item => <a key={item.href} href={item.href}
            aria-current={(item.href.startsWith("#") ? location.endsWith(item.href) : location === item.href) ? "page" : undefined}>{item.label}</a>)}
        </nav>}
      </div></div></div><div className="sarmg-header-actions" role="group" aria-label="全局操作">
        <div ref={setHeaderActions} style={{ display: "contents" }} /><ThemeToggle />
        <IconButton disabled={logoutPending} aria-label={logoutPending ? "正在退出…" : "退出"} title={logoutPending ? "正在退出…" : "退出"} onClick={() => void logout()}>{workspace.headerControls === "icons" ? <WorkspaceIcon name="logout" /> : "退出"}</IconButton>
      </div></PageHeader>
      {toasts.length > 0 && <div className="sarmg-toast-stack" role="region" aria-label="Notifications">{toasts.map(toast =>
        <Toast key={toast.id}><span>{toast.message}</span>
          <IconButton aria-label="Dismiss notification" onClick={() => setToasts(current => current.filter(item => item.id !== toast.id))}>×</IconButton>
        </Toast>)}</div>}
      <div className="sarmg-shell-layout sarmg-shell-layout--full"><main id="sarmg-main-content" className="sarmg-shell-main" tabIndex={-1}>
        {logoutError !== null && <ErrorState requestId={errorRequestId(logoutError)}>Sign out could not be confirmed. Try again.</ErrorState>}
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
    <h1>Administrator sign in</h1>
    {failure && <ErrorState requestId={failure.requestId}>Sign in failed. Check your credentials and try again.</ErrorState>}
    <FormField label="Username"><TextField name="username" autoComplete="username" required maxLength={64} readOnly={pending} /></FormField>
    <FormField label="Password"><TextField name="password" type="password" autoComplete="current-password" required maxLength={1024} readOnly={pending} /></FormField>
    <Button type="submit" disabled={pending}>{pending ? "Signing in…" : "Sign in"}</Button>
  </form>;
}

function ThemeToggle() {
  const [theme, setTheme] = useState(() => typeof window !== "undefined" && window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light");
  useEffect(() => {
    const root = document.documentElement;
    const previous = root.dataset.theme;
    root.dataset.theme = theme;
    return () => { if (previous === undefined) delete root.dataset.theme; else root.dataset.theme = previous; };
  }, [theme]);
  const label = theme === "light" ? "切换到深色模式" : "切换到浅色模式";
  return <IconButton aria-label={label} title={label} onClick={() => setTheme(current => current === "light" ? "dark" : "light")}>
    <svg aria-hidden="true" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      {theme === "light" ? <path d="M20.8 13A9 9 0 0 1 11 3.2 9 9 0 1 0 20.8 13Z" /> : <><circle cx="12" cy="12" r="4" /><path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M5 19l1.4-1.4M17.6 6.4 19 5" /></>}
    </svg>
  </IconButton>;
}
