import react from "@vitejs/plugin-react";
import { defineConfig, type UserConfig } from "vite";

export type SarmgReactViteOptions = {
  base?: string;
};

/** The exact React/Vite build baseline used by every non-Dufs Sarmg Web app. */
export function createSarmgReactViteConfig(
  options: SarmgReactViteOptions = {},
): UserConfig {
  return defineConfig({
    ...(options.base === undefined ? {} : { base: options.base }),
    plugins: [react()],
    build: {
      outDir: "dist",
      emptyOutDir: true,
    },
  });
}
