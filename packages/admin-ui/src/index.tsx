import {
  useEffect, useId, useRef,
  type ButtonHTMLAttributes, type HTMLAttributes, type InputHTMLAttributes,
  type ReactNode, type SelectHTMLAttributes, type TableHTMLAttributes,
} from "react";

function classes(base: string, extra?: string) { return extra ? `${base} ${extra}` : base; }

export function Button({ type = "button", className, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  return <button {...props} type={type} className={classes("sarmg-button", className)} />;
}
export function IconButton({ "aria-label": label, ...props }: ButtonHTMLAttributes<HTMLButtonElement>) {
  if (!label?.trim()) throw new TypeError("IconButton requires aria-label");
  return <Button {...props} aria-label={label} />;
}
export function TextField({ className, ...props }: InputHTMLAttributes<HTMLInputElement>) {
  return <input {...props} className={classes("sarmg-input", className)} />;
}
export function Select({ className, ...props }: SelectHTMLAttributes<HTMLSelectElement>) {
  return <select {...props} className={classes("sarmg-input", className)} />;
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
    dialog.querySelector<HTMLElement>("[data-sarmg-initial-focus]")?.focus();
    return () => {
      dialog.close();
      if (previous instanceof HTMLElement && previous.isConnected) previous.focus();
    };
  }, []);
  return <dialog ref={reference} className="sarmg-dialog" aria-labelledby={titleId} tabIndex={-1}
    aria-describedby={description ? descriptionId : undefined}
    onKeyDown={event => {
      if (event.key !== "Tab") return;
      const dialog = reference.current!;
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
    <div className="sarmg-dialog-heading"><h2 id={titleId}>{title}</h2>
      <IconButton aria-label="Close dialog" onClick={onClose}>×</IconButton></div>
    {description && <p id={descriptionId}>{description}</p>}{children}
  </dialog>;
}
export function ConfirmDangerDialog({ title, description, onConfirm, onClose, pending = false, children }: {
  title: string; description?: string; onConfirm: () => void; onClose: () => void; pending?: boolean; children?: ReactNode;
}) {
  return <Dialog title={title} description={description} onClose={() => { if (!pending) onClose(); }}>
    {children}
    <div className="sarmg-actions">
      <Button data-sarmg-initial-focus disabled={pending} onClick={onClose}>Cancel</Button>
      <Button className="sarmg-danger" disabled={pending} onClick={onConfirm}>{pending ? "Working…" : "Confirm"}</Button>
    </div>
  </Dialog>;
}
export function Toast({ className, ...props }: HTMLAttributes<HTMLDivElement>) {
  return <div {...props} role="status" aria-live="polite" aria-atomic="true" className={classes("sarmg-toast", className)} />;
}
export function StatusBadge({ status }: { status: string }) {
  return <span className="sarmg-status">{status}</span>;
}
export function Table({ className, ...props }: TableHTMLAttributes<HTMLTableElement>) {
  return <div className="sarmg-table-scroll" tabIndex={0} role="region" aria-label={props["aria-label"] ?? "Data table"}>
    <table {...props} className={classes("sarmg-table", className)} />
  </div>;
}
export function FormField({ label, children }: { label: string; children: ReactNode }) {
  return <label className="sarmg-form-field"><span>{label}</span>{children}</label>;
}
export function EmptyState({ children }: { children: ReactNode }) {
  return <div className="sarmg-empty">{children}</div>;
}
export function RequestId({ value }: { value?: string | null }) {
  return value && /^[A-Za-z0-9._:-]{1,128}$/.test(value)
    ? <p className="sarmg-request-id">Request ID: <code>{value}</code></p> : null;
}
export function ErrorState({ children, requestId, onRetry }: {
  children: ReactNode; requestId?: string | null; onRetry?: () => void;
}) {
  return <div className="sarmg-error" role="alert"><div>{children}</div>
    <RequestId value={requestId} />{onRetry && <Button onClick={onRetry}>Try again</Button>}
  </div>;
}
export function LoadingState({ children = "Loading…" }: { children?: ReactNode }) {
  return <div className="sarmg-loading" role="status" aria-live="polite">{children}</div>;
}
export function PageHeader({ children }: { children: ReactNode }) {
  return <header className="sarmg-page-header">{children}</header>;
}
