import { t } from "@sarmg/admin-ui/i18n";
import { createContext, useContext, type ReactNode, type InputHTMLAttributes } from "react";
import { createPortal } from "react-dom";
import { Button, IconButton, TextField } from "@sarmg/admin-ui";
import { DEFAULT_WORKSPACE_CONFIG, WORKSPACE_ICON_PATHS, validInstanceName, type WorkspaceConfig } from "./workspace-config.js";
export const WorkspaceContext = createContext<WorkspaceConfig>(DEFAULT_WORKSPACE_CONFIG);
export const HeaderActionsContext = createContext<HTMLElement | null>(null);
export const HeaderNavigationContext = createContext<HTMLElement | null>(null);
export function HeaderNavigation({ children, label = t("页面导航", "Page navigation") }: { children: ReactNode; label?: string }) {
  const target = useContext(HeaderNavigationContext);
  return target ? createPortal(<nav className="sarmg-header-navigation" aria-label={label}>{children}</nav>, target) : null;
}
export function HeaderActions({ children }: { children: ReactNode }) {
  const target = useContext(HeaderActionsContext);
  return target ? createPortal(children, target) : null;
}
export function WorkspaceIcon({ name }: { name: keyof typeof WORKSPACE_ICON_PATHS }) {
  return <svg aria-hidden="true" width="22" height="22" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round"><path d={WORKSPACE_ICON_PATHS[name]} /></svg>;
}
export function InstanceHeaderActions({ create, refresh, refreshing = false, createLabel = t("新建实例", "Create instance"), refreshLabel = t("刷新", "Refresh") }: {
  create?: () => void; refresh?: () => void; refreshing?: boolean; createLabel?: string; refreshLabel?: string;
}) {
  const config = useContext(WorkspaceContext);
  return <HeaderActions>
    {create && <IconButton aria-label={createLabel} title={createLabel} onClick={create}>{config.headerControls === "icons" ? <WorkspaceIcon name="create" /> : createLabel}</IconButton>}
    {refresh && <IconButton aria-label={refreshLabel} title={refreshLabel} onClick={refresh} disabled={refreshing}>{config.headerControls === "icons" ? <WorkspaceIcon name="refresh" /> : refreshLabel}</IconButton>}
  </HeaderActions>;
}
export function InstanceWorkspace({ instances, selected, select, label = t("实例", "Instances"), showSidebar = true, children }: {
  instances: readonly { id: string; name: string }[]; selected?: string | null; select(id: string): void; label?: string; showSidebar?: boolean; children: ReactNode;
}) {
  const config = useContext(WorkspaceContext);
  const sidebarVisible = showSidebar && instances.length > 0;
  return <div className={config.layout === "instances" ? `sarmg-instance-workspace${sidebarVisible ? "" : " sarmg-instance-workspace--full"}` : "sarmg-custom-workspace"}>
    {sidebarVisible && <aside className="sarmg-instance-sidebar" aria-label={label}><div className="sarmg-instance-list">{instances.map(item =>
      <Button key={item.id} title={item.name} aria-label={t("选择实例 {0}", "Select instance {0}", [item.name])} aria-pressed={selected === item.id} onClick={() => select(item.id)}><span>{item.name}</span></Button>
    )}</div></aside>}
    <section className="sarmg-content-stack" aria-label={t("实例详情与设置", "Instance details and settings")}>{children}</section>
  </div>;
}
/** Count Unicode scalar values, matching Rust chars(); do not use UTF-16 maxLength. */
export function InstanceNameField({ onChange, onInput, ...props }: Omit<InputHTMLAttributes<HTMLInputElement>, "maxLength">) {
  const config = useContext(WorkspaceContext);
  const validate = (input: HTMLInputElement) => input.setCustomValidity(validInstanceName(input.value, config.instanceNameMaxCharacters) ? "" : t("名称须为 1–{0} 个字符，不能包含控制字符", "Use 1–{0} characters without control characters", [config.instanceNameMaxCharacters]));
  return <TextField {...props} required pattern={`[^\\x00-\\x1f\\x7f]{1,${config.instanceNameMaxCharacters}}`} onChange={event => { validate(event.currentTarget); onChange?.(event); }} onInput={event => { validate(event.currentTarget); onInput?.(event); }} />;
}
