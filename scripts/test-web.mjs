import assert from "node:assert/strict";
import { readFileSync, readdirSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";
const root = fileURLToPath(new URL("../", import.meta.url));
const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
const built = await import(new URL("../dist/index.js", import.meta.url));
assert.equal(built.version, manifest.version, "built root export must match the package version");
assert.equal(readFileSync(join(root, "dist/index.d.ts"), "utf8"), `export declare const version: ${JSON.stringify(manifest.version)};\n`);
for (const name of ["contracts", "design-tokens", "web-fonts", "web-toolchain", "http-client", "admin-ui", "admin-web", "admin-shell"]) {
  const directory = join(root, "web", name);
  if (name === "web-fonts") {
    const result = spawnSync(process.execPath, [join(directory, "scripts/verify.mjs")], { cwd: directory, stdio: "inherit" });
    if (result.status !== 0) process.exit(result.status ?? 1);
  }
  const tests = readdirSync(join(directory, "test")).filter(name => name.endsWith(".test.mjs")).sort().map(name => join(directory, "test", name));
  const result = spawnSync(process.execPath, ["--test", ...tests], { cwd: directory, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

const types = spawnSync(process.execPath, [join(root, "node_modules/typescript/bin/tsc"), "--noEmit", "--strict", "--skipLibCheck", "--module", "NodeNext", "--moduleResolution", "NodeNext", "--target", "ES2022", join(root, "web/admin-shell/test/workspace-types.ts")], { cwd: root, stdio: "inherit" });
if (types.error) throw types.error;
if (types.status !== 0) process.exit(types.status ?? 1);
