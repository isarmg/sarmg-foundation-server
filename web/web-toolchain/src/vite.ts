import react from "@vitejs/plugin-react";
import { defineConfig, type UserConfig } from "vite";
import { assetBudget } from "./asset-budget.js";

export function createXcssReactViteConfig(options: { base?: string; maxAssetBytes?: number } = {}): UserConfig {
  const maximum = options.maxAssetBytes ?? 512 * 1024;
  return defineConfig({
    ...(options.base === undefined ? {} : { base: options.base }),
    resolve: { dedupe: ["react", "react-dom"] },
    plugins: [react(), assetBudget(maximum)],
    build: { outDir: process.env.XCSS_WEB_DIST ?? "dist", emptyOutDir: true, sourcemap: false, chunkSizeWarningLimit: Math.floor(maximum / 1024) },
  });
}
