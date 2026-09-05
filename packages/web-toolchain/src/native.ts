import { defineConfig, type UserConfig } from "vite";
import { assetBudget } from "./asset-budget.js";

/** Native ESM platform module for compile-time embedding; no React entry or runtime. */
export function createSarmgNativeModuleViteConfig(options: { entry: string; outDir: string }): UserConfig {
  return defineConfig({
    base: "./",
    plugins: [assetBudget(256 * 1024)],
    build: {
      outDir: options.outDir, emptyOutDir: true, sourcemap: false, assetsInlineLimit: 0, cssCodeSplit: false,
      rollupOptions: {
        input: options.entry, preserveEntrySignatures: "strict",
        output: { format: "es", entryFileNames: "platform.js", chunkFileNames: "chunks/[hash].js",
          assetFileNames: asset => asset.names.some(name => name.endsWith(".css")) ? "platform.css" : "[name][extname]" },
      },
    },
  });
}
