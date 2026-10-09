import { useEffect, useState } from "react";
import { t } from "./i18n.js";

export type CalendarDateRange = { start: string; end: string };
type Parts = [string, string, string];
type Errors = [boolean, boolean, boolean];

/** Gregorian calendar validation without browser-local time or Date rollover. */
export function calendarDateErrors([year, month, day]: Parts): Errors {
  const y = Number(year), m = Number(month), d = Number(day);
  const invalidYear = !/^\d{1,4}$/.test(year) || y < 1 || y > 9999;
  const invalidMonth = !/^\d{1,2}$/.test(month) || m < 1 || m > 12;
  const leap = y % 4 === 0 && (y % 100 !== 0 || y % 400 === 0);
  const days = invalidYear || invalidMonth ? 31 : [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31][m - 1]!;
  return [invalidYear, invalidMonth, !/^\d{1,2}$/.test(day) || d < 1 || d > days];
}
function canonical(parts: Parts): string {
  return parts.map((part, index) => part.padStart(index === 0 ? 4 : 2, "0")).join("-");
}
function split(value: string): Parts {
  const parts = value.split("-");
  return [parts[0] ?? "", parts[1] ? String(Number(parts[1])) : "", parts[2] ? String(Number(parts[2])) : ""];
}
export function isCalendarDate(value: string): boolean {
  return /^\d{4}-\d{2}-\d{2}$/.test(value) && !calendarDateErrors(value.split("-") as Parts).some(Boolean);
}
export function isCalendarDateRange(value: CalendarDateRange): boolean {
  return isCalendarDate(value.start) && isCalendarDate(value.end) && value.start <= value.end;
}
/** Separate editable numbers, with literal separators that never inherit errors.
 * Drafts stay local until Enter applies a complete, valid inclusive range.
 */
export function DateRangeField({ id, value, onApply, disabled = false, onValidityChange, serverInvalid = false }: {
  id: string; value: CalendarDateRange | null; onApply(value: CalendarDateRange): void;
  disabled?: boolean; onValidityChange?(valid: boolean): void; serverInvalid?: boolean;
}) {
  const [draft, setDraft] = useState<[Parts, Parts]>(() => [split(value?.start ?? ""), split(value?.end ?? "")]);
  const [editing, setEditing] = useState(false);
  useEffect(() => {
    setDraft([split(value?.start ?? ""), split(value?.end ?? "")]); setEditing(false);
  }, [value?.start, value?.end]);
  const errors = draft.map(calendarDateErrors);
  const reversed = !errors.some(parts => parts.some(Boolean)) && canonical(draft[0]) > canonical(draft[1]);
  const valid = !errors.some(parts => parts.some(Boolean)) && !reversed;
  useEffect(() => { onValidityChange?.(valid); }, [valid, onValidityChange]);
  const message = serverInvalid ? t("服务器无法查询此日期范围，请修改日期。", "The server cannot query this date range. Edit the dates.")
    : reversed ? t("结束日期不能早于开始日期。", "The end date must not precede the start date.")
    : t("请输入有效的年月日。", "Enter a valid year, month and day.");
  const showError = serverInvalid || editing && !valid;
  return <div className="xcss-date-range" id={id}>
    <div className="xcss-date-range-fields" role="group" aria-label={t("日期范围", "Date range")} onKeyDown={event => {
      if (event.key !== "Enter" || event.nativeEvent.isComposing || disabled) return;
      event.preventDefault(); setEditing(true);
      if (valid) onApply({ start: canonical(draft[0]), end: canonical(draft[1]) });
    }}>
      {draft.map((parts, endpoint) => <span className="xcss-date-range-endpoint" key={endpoint}>
        {endpoint === 1 && <span className="xcss-date-range-separator" aria-hidden="true">-</span>}
        {parts.map((part, field) => <span className="xcss-date-range-part" key={field}>
          {field > 0 && <span className="xcss-date-range-separator" aria-hidden="true">/</span>}
          <input id={`${id}-${endpoint === 0 ? "start" : "end"}-${["year", "month", "day"][field]}`}
            className="xcss-date-range-number" type="text" inputMode="numeric" autoComplete="off" spellCheck={false}
            size={field === 0 ? 4 : 2} maxLength={field === 0 ? 4 : 2} value={part} disabled={disabled}
            aria-label={`${endpoint === 0 ? t("开始日期", "Start date") : t("结束日期", "End date")} ${[t("年", "Year"), t("月", "Month"), t("日", "Day")][field]}`}
            aria-invalid={serverInvalid || editing && (errors[endpoint]![field] || reversed) || undefined}
            aria-describedby={showError ? `${id}-error` : undefined}
            onChange={event => { const next = draft.map(parts => [...parts]) as [Parts, Parts]; next[endpoint]![field] = event.target.value; setDraft(next); setEditing(true); }} />
        </span>)}
      </span>)}
    </div>
    {showError && <p id={`${id}-error`} className="xcss-date-range-error" role="alert">{message}</p>}
  </div>;
}
