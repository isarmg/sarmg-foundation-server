import { IconButton } from "@xcss/web/admin-ui";
import { t } from "@xcss/web/admin-ui/i18n";
import { useSyncExternalStore, type ReactNode } from "react";

import { getTheme, subscribeTheme, toggleTheme } from "./theme-store.js";

export function ThemeToggle() {
  const current = useSyncExternalStore(subscribeTheme, getTheme, () => "light");
  const label = current === "light" ? t("切换到深色模式", "Switch to dark mode") : t("切换到浅色模式", "Switch to light mode");
  return <IconButton aria-label={label} title={label} onClick={toggleTheme}>
    <svg aria-hidden="true" width="1em" height="1em" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round">
      {current === "light" ? <path d="M20.8 13A9 9 0 0 1 11 3.2 9 9 0 1 0 20.8 13Z" /> : <>
        <circle cx="12" cy="12" r="4" />
        <path d="M12 2v2m0 16v2M2 12h2m16 0h2M5 5l1.4 1.4m11.2 11.2L19 19M5 19l1.4-1.4M17.6 6.4 19 5" />
      </>}
    </svg>
  </IconButton>;
}

export function LoginControls({ children }: { children: ReactNode }) {
  return <div className="xcss-login-controls" role="group" aria-label={t("显示设置", "Display settings")}>
    {children}<ThemeToggle />
  </div>;
}
