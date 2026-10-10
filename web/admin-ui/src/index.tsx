import { t } from "./i18n.js";
export { DateRangeField, calendarDateErrors, isCalendarDate, isCalendarDateRange, type CalendarDateRange } from "./DateRangeField.js";
import { validationMessage } from "./i18n.js";
import {
  useEffect, useId, useRef,
  type ComponentProps,
  type ButtonHTMLAttributes, type HTMLAttributes, type InputHTMLAttributes,
  type ReactNode, type SelectHTMLAttributes, type TableHTMLAttributes,
} from "react";

function classes(base: string, extra?: string) { return extra ? `${base} ${extra}` : base; }

export function Button({ type = "button", className, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...props} type={type} className={classes("xcss-button", className)} />;
}
export function IconButton({ "aria-label": label, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  if (!label?.trim()) throw new TypeError("IconButton requires aria-label");
  return <Button {...props} aria-label={label} />;
}
export function TextField({ className, onInvalid, onInput, ...props }: ComponentProps<"input">) {
  const invalidField = useRef<{ field: HTMLInputElement; message: string } | null>(null);
  useEffect(() => {
    const invalid = invalidField.current;
    if (invalid && invalid.field.validationMessage === invalid.message) invalid.field.setCustomValidity("");
    invalidField.current = null;
  }, [props.value]);
  return <input {...props} className={classes("xcss-input", className)} onInvalid={event => {
    if (!event.currentTarget.validity.customError) {
      const message = validationMessage(event.currentTarget);
      event.currentTarget.setCustomValidity(message);
      invalidField.current = { field: event.currentTarget, message };
    }
    onInvalid?.(event);
  }} onInput={event => { invalidField.current = null; event.currentTarget.setCustomValidity(""); onInput?.(event); }} />;
}
export function Select({ className, onInvalid, onChange, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  const invalidField = useRef<{ field: HTMLSelectElement; message: string } | null>(null);
  useEffect(() => {
    const invalid = invalidField.current;
    if (invalid && invalid.field.validationMessage === invalid.message) invalid.field.setCustomValidity("");
    invalidField.current = null;
  }, [props.value]);
  return <select {...props} className={classes("xcss-input", className)} onInvalid={event => {
    if (!event.currentTarget.validity.customError) {
      const message = validationMessage(event.currentTarget);
      event.currentTarget.setCustomValidity(message);
      invalidField.current = { field: event.currentTarget, message };
    }
    onInvalid?.(event);
  }} onChange={event => { invalidField.current = null; event.currentTarget.setCustomValidity(""); onChange?.(event); }} />;
}
export function Checkbox(props: Omit<InputHTMLAttributes<HTMLInputElement>, "type">) {
  return <input {...props} type="checkbox" />;
}

export type DialogProps = {
  title: string; description?: string; children: ReactNode; onClose: () => void;
};
/** Native modal semantics supply focus containment, background inertness and Escape. */
export function Dialog({ title, description, children, onClose }: DialogProps) {
  const reference = useRef<HTMLDialogElement>(null);
  const titleId = useId();
  const descriptionId = useId();
  useEffect(() => {
    const dialog = reference.current!;
    const previous = document.activeElement;
    dialog.showModal();
    // React's autoFocus runs before showModal and is not a native autofocus
    // attribute. Apply the safe initial target after the dialog is opened.
    dialog.querySelector<HTMLElement>("[data-xcss-initial-focus]")?.focus();
    return () => {
      dialog.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  return <dialog ref={reference} className="xcss-dialog" aria-labelledby={titleId} tabIndex={-1}
    aria-describedby={description ? descriptionId : undefined}
    onKeyDown={event => {
      if (event.key !== "Tab") return;
      const dialog = reference.current!;
      // Native media controls have focusable subwidgets outside the light DOM.
      if (dialog.querySelector("video[controls],audio[controls]")) return;
      const focusable = Array.from(dialog.querySelectorAll<HTMLElement>(
        'button,input,select,textarea,a[href],[tabindex]',
      )).filter(element => element.tabIndex >= 0 && !element.matches(':disabled,[hidden]') && element.getClientRects().length > 0);
      const first = focusable[0];
      const last = focusable.at(-1);
      if (!first) { event.preventDefault(); dialog.focus(); }
      else if (event.shiftKey && (document.activeElement === first || document.activeElement === dialog)) {
        event.preventDefault(); last!.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault(); first.focus();
      }
    }}
    onCancel={(event) => { event.preventDefault(); onClose(); }}>
    <div className="xcss-dialog-heading"><h2 id={titleId}>{title}</h2>
      <IconButton aria-label={t("关闭对话框", "Close dialog")} onClick={onClose}>×</IconButton></div>
    {description && <p id={descriptionId}>{description}</p>}{children}
  </dialog>;
}
export function ConfirmDangerDialog({ title, description, onConfirm, onClose, pending = false, children }: {
  title: string; description?: string; onConfirm: () => void; onClose: () => void; pending?: boolean; children?: ReactNode;
}) {
  return <Dialog title={title} description={description} onClose={() => { if (!pending) onClose(); }}>
    {children}
    <div className="xcss-actions">
      <Button data-xcss-initial-focus disabled={pending} onClick={onClose}>{t("取消", "Cancel")}</Button>
      <Button className="xcss-danger" disabled={pending} onClick={onConfirm}>{pending ? t("正在处理…", "Working…") : t("确认", "Confirm")}</Button>
    </div>
  </Dialog>;
}
export function Toast({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div {...props} role="status" aria-live="polite" aria-atomic="true" className={classes("xcss-toast", className)} />;
}
export function StatusBadge({ status }: { status: string }) {
  return <span className="xcss-status">{status}</span>;
}
export function Table({ className, ...props }: TableHTMLAttributes<HTMLTableElement>) {
  return <div className="xcss-table-scroll" tabIndex={0} role="region" aria-label={props["aria-label"] ?? t("数据表格", "Data table")}>
    <table {...props} className={classes("xcss-table", className)} />
  </div>;
}
export function FormField({ label, children }: { label: string; children: ReactNode }) {
  return <label className="xcss-form-field"><span>{label}</span>{children}</label>;
}
export function EmptyState({ children }: { children: ReactNode }) {
  return <div className="xcss-empty">{children}</div>;
}
export function RequestId({ value }: { value?: string | null }) {
  return value && /^[A-Za-z0-9._:-]{1,128}$/.test(value)
    ? <p className="xcss-request-id">{t("请求标识：", "Request ID:")}<code>{value}</code></p> : null;
}
export function ErrorState({ children, requestId, onRetry, retryLabel }: {
  children: ReactNode; requestId?: string | null; onRetry?: () => void; retryLabel?: string;
}) {
  return <div className="xcss-error" role="alert"><div>{children}</div>
    <RequestId value={requestId} />{onRetry && <Button onClick={onRetry}>{retryLabel ?? t("重试", "Try again")}</Button>}
  </div>;
}
export function LoadingState({ children = t("正在加载…", "Loading…") }: { children?: ReactNode }) {
  return <div className="xcss-loading" role="status" aria-live="polite">{children}</div>;
}
export function PageHeader({ children }: { children: ReactNode }) {
  return <header className="xcss-page-header">{children}</header>;
}
