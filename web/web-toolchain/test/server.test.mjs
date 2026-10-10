import { test } from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { chmodSync, mkdirSync, mkdtempSync, writeFileSync, readFileSync, rmSync, symlinkSync, linkSync, realpathSync } from "node:fs";
import { execFileSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { assertWebOutput, buildWebServer, parseWebServerBuildConfig, requireServerBuildPlatform, verifyEmbeddedAssets } from "../../../dist/web-toolchain/server.js";

test("server build entry refuses client platforms, ARM64 and non-glibc Linux", () => {
  for (const [property, value] of [["platform", "win32"], ["arch", "arm64"]]) {
    const original = Object.getOwnPropertyDescriptor(process, property);
    try {
      Object.defineProperty(process, property, { value, configurable: true });
      assert.throws(requireServerBuildPlatform, /require Linux x86_64 with glibc/);
    } finally { Object.defineProperty(process, property, original); }
  }
  const originalReport = process.report.getReport;
  try {
    process.report.getReport = () => ({ header: {} });
    assert.throws(requireServerBuildPlatform, /require Linux x86_64 with glibc/);
  } finally { process.report.getReport = originalReport; }
  requireServerBuildPlatform();
});

const hash = value => createHash("sha256").update(value).digest("hex");
function fixture(run) {
  // Match the builder's canonical checkout identity, including macOS /var.
  // Unicode and spaces exercise subprocess path arguments in every environment.
  const directory = realpathSync(mkdtempSync(join(tmpdir(), "xcss-embedded 中文 acceptance-")));
  const html = "<!doctype html><title>current artifact</title>";
  writeFileSync(join(directory, "index.html"), html);
  const manifest = JSON.stringify({ format: "web-assets-v1", files: [{ path: "index.html", content_type: "text/html; charset=utf-8", size: Buffer.byteLength(html), sha256: hash(html) }] });
  try { run(directory, manifest); } finally { rmSync(directory, { recursive: true, force: true }); }
}
test("executable inventory accepts matching bytes and binds exact manifest", () => fixture((directory, manifest) => {
  assert.deepEqual(verifyEmbeddedAssets(manifest, directory), { files: 1, sha256: hash(manifest) });
  assert.deepEqual(verifyEmbeddedAssets(`${manifest}\n`, directory), { files: 1, sha256: hash(manifest) });
  assert.throws(() => verifyEmbeddedAssets(`${manifest}\n\n`, directory), /not canonical/);
}));
test("editing Web after compilation cannot pass artifact acceptance", () => fixture((directory, manifest) => {
  writeFileSync(join(directory, "index.html"), "an older or newer page");
  assert.throws(() => verifyEmbeddedAssets(manifest, directory), /Executable Web bytes differ/);
}));
test("asset omission and extra output cannot pass artifact acceptance", () => fixture((directory, manifest) => {
  writeFileSync(join(directory, "uncompiled.js"), "export default 1");
  assert.throws(() => verifyEmbeddedAssets(manifest, directory), /Executable omitted/);
}));
test("duplicate and unsorted inventory paths are rejected", () => fixture((directory, manifest) => {
  const value = JSON.parse(manifest); value.files.push(value.files[0]);
  assert.throws(() => verifyEmbeddedAssets(JSON.stringify(value), directory), /Invalid compiled asset/);
}));
test("linked inputs cannot be used to verify embedded outputs", () => fixture((directory, manifest) => {
  symlinkSync(join(directory, "index.html"), join(directory, "linked.html"));
  assert.throws(() => verifyEmbeddedAssets(manifest, directory), /Linked Web input/);
  rmSync(join(directory, "linked.html"));
  linkSync(join(directory, "index.html"), join(directory, "hardlink.html"));
  assert.throws(() => verifyEmbeddedAssets(manifest, directory), /Unsafe Web input/);
}));
test("declarations reject shell commands and unknown configuration keys", () => {
  const valid = { format: 1, web: { directory: "web", script: "build", dist: "dist" }, rust: { manifest: "Cargo.toml", package: "example", binary: "example", source_revision_env: "SOURCE_REVISION" } };
  assert.deepEqual(parseWebServerBuildConfig(valid), valid);
  assert.throws(() => parseWebServerBuildConfig({ ...valid, command: "sh" }), /Invalid build declaration/);
  assert.throws(() => parseWebServerBuildConfig({ ...valid, web: { ...valid.web, script: "build; echo unsafe" } }), /Invalid web.script/);
  assert.throws(() => parseWebServerBuildConfig({ ...valid, web: { ...valid.web, dist: "../src" } }), /canonical relative/);
  assert.throws(() => parseWebServerBuildConfig({ ...valid, rust: { ...valid.rust, manifest: "/etc/Cargo.toml" } }), /canonical relative/);
});
test("build output cannot replace source ancestors or another project", () => fixture(directory => {
  const web = join(directory, "web");
  const dist = join(web, "dist");
  assert.doesNotThrow(() => assertWebOutput(directory, web, dist, dist));
  assert.throws(() => assertWebOutput(directory, web, dist, directory), /ancestor/);
  assert.throws(() => assertWebOutput(directory, web, dist, realpathSync(tmpdir())), /ancestor/);
  assert.throws(() => assertWebOutput(directory, web, dist, join(directory, "src")), /declared output/);
  assert.throws(() => assertWebOutput(directory, web, dist, join(directory, ".git", "objects")), /source directory/);
  writeFileSync(join(directory, "package.json"), "{}");
  const unrelated = join(directory, "other");
  assert.throws(() => assertWebOutput(unrelated, unrelated, join(unrelated, "dist"), directory), /ancestor/);
  const separate = `${directory}-separate`;
  assert.throws(() => assertWebOutput(separate, separate, join(separate, "dist"), directory), /source metadata/);
  const foreign = join(directory, "foreign");
  const foreignSource = join(foreign, "src");
  mkdirSync(foreignSource, { recursive: true });
  writeFileSync(join(foreign, "package.json"), "{}");
  assert.throws(() => assertWebOutput(separate, separate, join(separate, "dist"), foreignSource), /another project/);
  const declaredNestedOutput = join(foreign, "dist");
  assert.doesNotThrow(() => assertWebOutput(directory, directory, declaredNestedOutput, declaredNestedOutput));
  mkdirSync(join(foreign, ".git"));
  assert.throws(() => assertWebOutput(directory, directory, declaredNestedOutput, declaredNestedOutput), /another project/);
}));
test("a declaration pointing at tracked source is rejected before the Web command runs", () => fixture(directory => {
  mkdirSync(join(directory, "web", "src"), { recursive: true });
  const source = join(directory, "web", "src", "page.js");
  writeFileSync(source, "keep this authored source");
  const declaration = { format: 1, web: { directory: "web", script: "must-not-run", dist: "src" }, rust: { manifest: "Cargo.toml", package: "example", binary: "example", source_revision_env: "SOURCE_REVISION" } };
  writeFileSync(join(directory, "xcss-web-build.json"), JSON.stringify(declaration));
  execFileSync("git", ["init", "--quiet", directory]);
  execFileSync("git", ["-C", directory, "add", "."]);
  execFileSync("git", ["-C", directory, "-c", "user.name=fixture", "-c", "user.email=fixture@example.invalid", "commit", "--quiet", "-m", "fixture"]);
  assert.throws(() => buildWebServer(join(directory, "xcss-web-build.json"), { mode: "development", webOnly: true, noInstall: true }), /tracked source files/);
  assert.equal(readFileSync(source, "utf8"), "keep this authored source");
}));
test("the actual build CLI orders Web before Cargo, accepts a development archive and emits only its executable path", () => fixture(directory => {
  const declaration = { format: 1, web: { directory: ".", script: "build", dist: "dist" }, rust: { manifest: "Cargo.toml", package: "example", binary: "example", source_revision_env: "SOURCE_REVISION" } };
  writeFileSync(join(directory, "xcss-web-build.json"), JSON.stringify(declaration));
  writeFileSync(join(directory, "package.json"), JSON.stringify({ scripts: { build: "node build-fixture.mjs" } }));
  writeFileSync(join(directory, "Cargo.toml"), '[package]\nname="example"\nversion="1.0.0"\n');
  writeFileSync(join(directory, "build-fixture.mjs"), `import {mkdirSync,writeFileSync,appendFileSync} from 'node:fs';
mkdirSync(process.env.XCSS_WEB_DIST,{recursive:true});writeFileSync(process.env.XCSS_WEB_DIST+'/index.html','current build');appendFileSync('calls','web\\n');console.log('Web build progress');`);
  const executable = join(directory, "server-fixture.mjs");
  const compiledInventory = JSON.stringify({ format: "web-assets-v1", files: [{ path: "index.html", content_type: "text/html; charset=utf-8", size: Buffer.byteLength("current build"), sha256: hash("current build") }] });
  writeFileSync(executable, `#!${process.execPath}\nprocess.stdout.write(${JSON.stringify(compiledInventory)});`);
  chmodSync(executable, 0o755);
  const command = join(directory, "commands"); mkdirSync(command);
  writeFileSync(join(command, "cargo"), `#!${process.execPath}\nimport {readFileSync,appendFileSync} from 'node:fs';
if(readFileSync(process.env.XCSS_WEB_DIST+'/index.html','utf8')!=='current build')process.exit(2);
if(process.env.SOURCE_REVISION!=='unbound')process.exit(3);appendFileSync('calls','cargo\\n');
console.log(JSON.stringify({reason:'compiler-artifact',target:{name:'example',kind:['bin']},executable:${JSON.stringify(executable)}}));`);
  chmodSync(join(command, "cargo"), 0o755);
  const cli = new URL("../../../dist/web-toolchain/server-cli.js", import.meta.url);
  const output = execFileSync(process.execPath, [fileURLToPath(cli), "--config", join(directory, "xcss-web-build.json"), "--mode", "development", "--no-install"], { cwd: directory, env: { ...process.env, PATH: `${command}:${process.env.PATH}` }, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] });
  assert.equal(output, `${executable}\n`);
  assert.equal(readFileSync(join(directory, "calls"), "utf8"), "web\ncargo\n");
}));
