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
export type InstancePage = "instances" | "details" | "logs";
export function InstancePageNavigation({ page, navigate, detailsDisabled = false }: {
  page: InstancePage; navigate(page: InstancePage): void; detailsDisabled?: boolean;
}) {
  const pages: readonly [InstancePage, string][] = [
    ["instances", t("实例列表", "Instance list")],
    ["details", t("详细信息", "Details")],
    ["logs", t("日志", "Logs")],
  ];
  return <HeaderNavigation label={t("实例工作区", "Instance workspace")}>{pages.map(([id, label]) =>
    <Button key={id} aria-pressed={page === id} disabled={id === "details" && detailsDisabled} onClick={() => navigate(id)}>{label}</Button>
  )}</HeaderNavigation>;
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
/** Count Unicode scalar values, matching Rust chars(); do not use UTF-16 maxLength. */
export function InstanceNameField({ onChange, onInput, ...props }: Omit<InputHTMLAttributes<HTMLInputElement>, "maxLength">) {
  const config = useContext(WorkspaceContext);
  const validate = (input: HTMLInputElement) => input.setCustomValidity(validInstanceName(input.value, config.instanceNameMaxCharacters) ? "" : t("名称须为 1–{0} 个字符，不能包含控制字符", "Use 1–{0} characters without control characters", [config.instanceNameMaxCharacters]));
  return <TextField {...props} required pattern={`[^\\x00-\\x1f\\x7f]{1,${config.instanceNameMaxCharacters}}`} onChange={event => { validate(event.currentTarget); onChange?.(event); }} onInput={event => { validate(event.currentTarget); onInput?.(event); }} />;
}
