import { rm, mkdir, readFile, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
import process from "node:process";
const root = fileURLToPath(new URL("../", import.meta.url));
if (process.platform !== "linux" || process.arch !== "x64" || !process.report.getReport().header.glibcVersionRuntime) {
  throw new Error("xcss server Web build inputs require Linux x86_64 with glibc");
}
const steps = [
  ["contracts", "tsconfig.json", "copy-contract-data.mjs"],
  ["design-tokens", "tsconfig.json", "copy-css.mjs"],
  ["web-fonts", null, "build.mjs"],
  ["web-toolchain", "tsconfig.build.json", "copy-config.mjs"],
  ["http-client", "tsconfig.json", null],
  ["admin-ui", "tsconfig.json", "copy-assets.mjs"],
  ["admin-web", "tsconfig.build.json", null],
  ["admin-shell", "tsconfig.json", null],
];
await rm(join(root, "dist"), { recursive: true, force: true });
await mkdir(join(root, "dist"));
function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}
for (const [name, config, assets] of steps) {
  const directory = join(root, "web", name);
  if (config) run(process.execPath, [join(root, "node_modules/typescript/bin/tsc"), "-p", config], directory);
  if (assets) run(process.execPath, [join(directory, "scripts", assets)], directory);
}
const { version } = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
await writeFile(join(root, "dist/index.js"), `export const version = ${JSON.stringify(version)};\n`);
await writeFile(join(root, "dist/index.d.ts"), `export declare const version: ${JSON.stringify(version)};\n`);
