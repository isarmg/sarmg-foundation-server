import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(new URL("../", import.meta.url));
for (const name of ["contracts", "design-tokens", "web-toolchain", "http-client", "admin-ui", "admin-web", "admin-shell"]) {
  const config = ["web-toolchain", "admin-web"].includes(name) ? "tsconfig.build.json" : "tsconfig.json";
  const result = spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"), "-p", config, "--noEmit"], { cwd: join(root, "web", name), stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
