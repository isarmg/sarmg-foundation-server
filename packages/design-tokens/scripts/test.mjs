import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";

const packageRoot = new URL("../", import.meta.url);
const staleFile = new URL("../dist/stale-build-output.txt", import.meta.url);

await mkdir(new URL("../dist/", import.meta.url), { recursive: true });
await writeFile(staleFile, "a clean build must remove this file\n", "utf8");

const npmExecutable = process.env.npm_execpath;
const command = npmExecutable ? process.execPath : "pnpm";
const args = npmExecutable
  ? [npmExecutable, "run", "build"]
  : ["run", "build"];
const build = spawnSync(command, args, {
  cwd: packageRoot,
  encoding: "utf8",
  stdio: "inherit",
});
assert.equal(build.error, undefined, `failed to start clean build: ${build.error}`);
assert.equal(build.status, 0, "design-token clean build failed");

const tests = spawnSync(process.execPath, ["--test", "test/*.test.mjs"], {
  cwd: packageRoot,
  encoding: "utf8",
  shell: false,
  stdio: "inherit",
});
assert.equal(tests.error, undefined, `failed to start tests: ${tests.error}`);
assert.equal(tests.status, 0, "design-token consistency tests failed");
