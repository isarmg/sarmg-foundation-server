export const tokens = {
  color: {
    blue500: "#3b82f6",
    blue600: "#2563eb",
    gray50: "#fafafa",
    gray100: "#f4f4f5",
    gray900: "#18181b",
    red600: "#dc2626",
  },
  space: {
    1: "4px",
    2: "8px",
    3: "12px",
    4: "16px",
    6: "24px",
    8: "32px",
  },
  radius: {
    sm: "6px",
    md: "8px",
    lg: "12px",
  },
  typography: {
    fontUi: '"Sarmg Maple", ui-monospace, monospace',
    fontMono: '"Sarmg Maple", ui-monospace, monospace',
    fontSizeBody: "1rem",
    lineHeightBody: "1.5",
  },
} as const;

/**
 * Semantic values are exported for build-time consumers that cannot resolve
 * CSS custom properties. Keep these values byte-for-byte aligned with the
 * effective values in tokens.css and tokens.dark.css.
 */
export const semanticTokens = {
  light: {
    actionPrimary: tokens.color.blue600,
    bgPage: tokens.color.gray50,
    bgPanel: "#ffffff",
    textPrimary: tokens.color.gray900,
    textDanger: tokens.color.red600,
    textLink: tokens.color.blue600,
  },
  dark: {
    actionPrimary: tokens.color.blue600,
    bgPage: "#09090b",
    bgPanel: "#18181b",
    textPrimary: "#fafafa",
    textDanger: "#f87171",
    textLink: "#60a5fa",
  },
} as const;
