import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "./tests/web", testMatch: "*.spec.mjs", fullyParallel: true,
  use: { baseURL: "http://127.0.0.1:4189", trace: "retain-on-failure" },
  projects: [{ name: "chromium", use: { browserName: "chromium" } }, { name: "firefox", use: { browserName: "firefox" } }],
  webServer: {
    command: "pnpm --filter @sarmg/web-toolchain exec vite --config ../../tests/web/vite.config.mjs --host 127.0.0.1 --port 4189 --strictPort",
    url: "http://127.0.0.1:4189", reuseExistingServer: false,
  },
});
