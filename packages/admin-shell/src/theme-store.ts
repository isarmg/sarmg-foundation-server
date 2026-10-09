export type Theme = "light" | "dark";
export const THEME_STORAGE_KEY = "sarmg:theme";
const systemTheme = typeof window === "undefined" ? undefined : window.matchMedia("(prefers-color-scheme: dark)");
const listeners = new Set<() => void>();

function savedTheme(): Theme | undefined {
  try {
    const saved = window.localStorage.getItem(THEME_STORAGE_KEY) ?? window.localStorage.getItem("xsos:theme");
    return saved === "light" || saved === "dark" ? saved : undefined;
  } catch { return undefined; }
}

let preference = savedTheme();
let theme: Theme = preference ?? (systemTheme?.matches ? "dark" : "light");

function applyTheme() {
  if (typeof document !== "undefined") document.documentElement.dataset.theme = theme;
  for (const listener of listeners) listener();
}

// Set the palette before the font gate opens, and keep it across login/logout.
applyTheme();
systemTheme?.addEventListener("change", event => {
  if (preference !== undefined) return;
  theme = event.matches ? "dark" : "light";
  applyTheme();
});

export function toggleTheme() {
  theme = theme === "light" ? "dark" : "light";
  preference = theme;
  try { window.localStorage.setItem(THEME_STORAGE_KEY, theme); } catch { /* Keep the choice for this document when storage is unavailable. */ }
  applyTheme();
}

export function subscribeTheme(listener: () => void) {
  listeners.add(listener);
  return () => { listeners.delete(listener); };
}

export function getTheme(): Theme { return theme; }
