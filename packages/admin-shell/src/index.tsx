import {
  Component, createContext, useCallback, useContext, useEffect, useRef, useState,
  type FormEvent, type ReactNode,
} from "react";
import {
  Button, Dialog, ErrorState, FormField, IconButton, LoadingState, PageHeader,
  RequestId, Select, StatusBadge, Table, TextField, Toast,
} from "@sarmg/admin-ui";
import {
  createAdministratorApiClient, isPlatformDiagnostics, PLATFORM_DIAGNOSTICS_PATH,
  type AdministratorApiClient, type PlatformDiagnostics,
} from "@sarmg/admin-web";
import { useAdministratorSession, type AdministratorSessionController } from "@sarmg/admin-web/react";

export { AdministratorsPanel } from "./administrators.js";

export type ProductIdentity = { name: string; version: string };
export type NavigationItem = { label: string; href: string };
export type AdminApplicationOptions = {
  product: ProductIdentity;
  client?: AdministratorApiClient;
  navigation: readonly NavigationItem[];
  routes: ReactNode;
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

export class ApplicationErrorBoundary extends Component<{ children: ReactNode }, { failed: boolean; requestId?: string }> {
  state: { failed: boolean; requestId?: string } = { failed: false };
  static getDerivedStateFromError(error: unknown) {
    return { failed: true, requestId: errorRequestId(error) };
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
  if (!options.product.name.trim() || !options.product.version.trim()) throw new TypeError("Product identity is required");
  const seen = new Set<string>();
  for (const item of options.navigation) {
    if (!item.label.trim() || !/^(?:\/(?!\/)|#)/.test(item.href) || /[\\\u0000-\u0020\u007f]/.test(item.href) || seen.has(item.href)) {
      throw new TypeError("Navigation requires unique local links and labels");
    }
    seen.add(item.href);
  }
  const client = options.client ?? createAdministratorApiClient();
  return function SarmgAdminApplication() {
    return <ApplicationErrorBoundary><AdminShell options={options} client={client} /></ApplicationErrorBoundary>;
  };
}

function AdminShell({ options, client }: { options: AdminApplicationOptions; client: AdministratorApiClient }) {
  const session = useAdministratorSession(client);
  const [diagnostics, setDiagnostics] = useState(false);
  const [logoutPending, setLogoutPending] = useState(false);
  const [logoutError, setLogoutError] = useState<unknown>(null);
  const [toasts, setToasts] = useState<{ id: number; message: string }[]>([]);
  const sequence = useRef(0);
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
    if (session.phase !== "authenticated") { setToasts([]); setDiagnostics(false); setLogoutError(null); }
  }, [session.phase]);
  const identity = <div className="sarmg-product-identity"><strong>{options.product.name}</strong><small>{options.product.version}</small></div>;
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
    <div className="sarmg-admin-shell">
      <a className="sarmg-skip-link" href="#sarmg-main-content" onClick={event => {
        event.preventDefault(); document.getElementById("sarmg-main-content")?.focus();
      }}>Skip to content</a>
      <PageHeader>{identity}<ThemeSelect />
        <Button onClick={() => setDiagnostics(true)}>Diagnostics</Button>
        <Button disabled={logoutPending} onClick={() => void logout()}>{logoutPending ? "Signing out…" : "Sign out"}</Button>
      </PageHeader>
      {toasts.length > 0 && <div className="sarmg-toast-stack" role="region" aria-label="Notifications">{toasts.map(toast =>
        <Toast key={toast.id}><span>{toast.message}</span>
          <IconButton aria-label="Dismiss notification" onClick={() => setToasts(current => current.filter(item => item.id !== toast.id))}>×</IconButton>
        </Toast>)}</div>}
      <div className="sarmg-shell-layout"><nav className="sarmg-navigation" aria-label="Product navigation">
        {options.navigation.map(item => <a key={item.href} href={item.href}
          aria-current={(item.href.startsWith("#") ? location.endsWith(item.href) : location === item.href) ? "page" : undefined}>{item.label}</a>)}
      </nav><main id="sarmg-main-content" className="sarmg-shell-main" tabIndex={-1}>
        {logoutError !== null && <ErrorState requestId={errorRequestId(logoutError)}>Sign out could not be confirmed. Try again.</ErrorState>}
        <ApplicationErrorBoundary key={location}>{options.routes}</ApplicationErrorBoundary>
      </main></div>
      {diagnostics && <Dialog title="Platform diagnostics" onClose={() => setDiagnostics(false)}>
        <DiagnosticsPanel client={client} />
      </Dialog>}
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

function ThemeSelect() {
  const [theme, setTheme] = useState("system");
  useEffect(() => {
    const root = document.documentElement;
    const previous = root.dataset.theme;
    const preference = window.matchMedia("(prefers-color-scheme: dark)");
    const apply = () => { root.dataset.theme = theme === "system" ? (preference.matches ? "dark" : "light") : theme; };
    apply(); preference.addEventListener("change", apply);
    return () => { preference.removeEventListener("change", apply); if (previous === undefined) delete root.dataset.theme; else root.dataset.theme = previous; };
  }, [theme]);
  return <label className="sarmg-theme-select"><span>Theme</span><Select value={theme} onChange={event => setTheme(event.target.value)}>
    <option value="system">System</option><option value="light">Light</option><option value="dark">Dark</option>
  </Select></label>;
}

export function DiagnosticsPanel({ client }: { client: AdministratorApiClient }) {
  const [value, setValue] = useState<PlatformDiagnostics | null>(null);
  const [failure, setFailure] = useState<{ requestId?: string } | null>(null);
  const [generation, setGeneration] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setValue(null); setFailure(null);
    void client.request(PLATFORM_DIAGNOSTICS_PATH, isPlatformDiagnostics, { signal: controller.signal })
      .then(result => { if (!controller.signal.aborted) setValue(result); })
      .catch(error => { if (!controller.signal.aborted) setFailure({ requestId: errorRequestId(error) }); });
    return () => controller.abort();
  }, [client, generation]);
  if (failure) return <ErrorState requestId={failure.requestId} onRetry={() => setGeneration(current => current + 1)}>Diagnostics are unavailable.</ErrorState>;
  if (!value) return <LoadingState>Loading diagnostics…</LoadingState>;
  return <section className="sarmg-diagnostics" aria-label="Platform diagnostics details">
    <Button onClick={() => setGeneration(current => current + 1)}>Refresh</Button>
    <RequestId value={value.request_id} />
    <dl><dt>Product</dt><dd>{value.product.id} {value.product.version}</dd>
      <dt>Foundation revision</dt><dd><code>{value.product.foundation_revision}</code></dd>
      <dt>Profile</dt><dd>{value.product.profile}</dd>
      <dt>Readiness</dt><dd><StatusBadge status={value.health.ready ? "Ready" : "Not ready"} /></dd>
      <dt>Health</dt><dd>{!value.health.live ? "Unhealthy" : value.health.degraded ? "Degraded" : "Healthy"}</dd>
      <dt>Schema</dt><dd>{value.schema_identity ? <>{value.schema_identity.schema_revision} · <code>{value.schema_identity.schema_sha256}</code></> : "Not applicable"}</dd>
    </dl>
    <Table aria-label="Health checks"><caption>Health checks</caption><thead><tr><th scope="col">Check</th><th scope="col">Result</th></tr></thead>
      <tbody>{Object.entries(value.checks).map(([name, passed]) => <tr key={name}><th scope="row">{name}</th><td>{passed ? "Passed" : "Failed"}</td></tr>)}</tbody></Table>
    <Table aria-label="Background tasks"><caption>Background tasks</caption><thead><tr><th scope="col">Task</th><th scope="col">Criticality</th><th scope="col">State</th></tr></thead>
      <tbody>{Object.entries(value.tasks).map(([name, task]) => <tr key={name}><th scope="row">{name}</th><td>{task.criticality}</td><td>{task.state}</td></tr>)}</tbody></Table>
    <Table aria-label="Backlog"><caption>Backlog (unavailable values are not zero)</caption><thead><tr><th scope="col">Metric</th><th scope="col">Value</th></tr></thead>
      <tbody>{Object.entries(value.metrics).map(([name, count]) => <tr key={name}><th scope="row">{name}</th><td>{count === null ? "Unavailable / not applicable" : count}</td></tr>)}</tbody></Table>
  </section>;
}
