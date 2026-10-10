import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, lstatSync, readFileSync, readdirSync, realpathSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve, sep } from "node:path";
import { WEB_TOOLCHAIN } from "./index.js";

export interface WebServerBuildConfig {
  format: 1;
  web: { directory: string; script: string; dist: string };
  rust: { manifest: string; package: string; binary: string; source_revision_env: string };
}
export interface WebServerBuildOptions {
  mode: "development" | "release";
  dist?: string;
  webOnly?: boolean;
  rustOnly?: boolean;
  noInstall?: boolean;
  sourceRevision?: string;
  cargoArgs?: string[];
}
interface AssetFile { path: string; content_type: string; size: number; sha256: string }

function within(root: string, path: string): string {
  if (!path || isAbsolute(path)) throw new Error(`Expected a relative project path: ${path}`);
  const result = resolve(root, path);
  const rel = relative(root, result);
  if (rel.startsWith(`..${sep}`) || rel === "..") throw new Error(`Path escapes project: ${path}`);
  return result;
}
function canonicalPath(value: unknown, allowRoot = false): value is string {
  return typeof value === "string" && ((allowRoot && value === ".") ||
    (!isAbsolute(value) && !value.includes("\\") && value.split("/").every(part => part !== "" && part !== "." && part !== ".." && !/[\x00-\x1f\x7f]/.test(part))));
}
function safeOutput(directory: string): void {
  for (let path = directory; ; path = dirname(path)) {
    const metadata = lstatSync(path, { throwIfNoEntry: false });
    if (metadata && (!metadata.isDirectory() || metadata.isSymbolicLink())) throw new Error(`Web output has a linked or non-directory parent: ${path}`);
    if (path === dirname(path)) break;
  }
}
/** A generated output must never replace its source tree, Git metadata, or another project. */
export function assertWebOutput(root: string, web: string, defaultDist: string, dist: string): void {
  const contains = (parent: string, child: string): boolean => {
    const path = relative(parent, child);
    return path === "" || (path !== ".." && !path.startsWith(`..${sep}`) && !isAbsolute(path));
  };
  if (contains(dist, root) || contains(dist, web) || dist.split(sep).includes(".git")) throw new Error("Web output may not replace a source directory or its ancestor");
  if (dist !== defaultDist && contains(root, dist)) throw new Error("A custom Web output inside the project must equal the declared output");
  safeOutput(dist);
  if ([".git", "Cargo.toml", "package.json"].some(name => existsSync(resolve(dist, name)))) throw new Error("Web output contains project source metadata");
  for (let parent = dirname(dist); ; parent = dirname(parent)) {
    // The declared checkout and Web package are the only permitted source ancestors.
    if (parent === root || parent === web) break;
    const ownDeclaredOutput = dist === defaultDist && contains(root, dist);
    // A package can contain nested ESM or Cargo packages. Only the explicit,
    // source-checked default output may pass those owned metadata directories.
    if (existsSync(resolve(parent, ".git")) || (!ownDeclaredOutput &&
        ["Cargo.toml", "package.json"].some(name => existsSync(resolve(parent, name))))) throw new Error("Web output is inside another project source tree");
    if (parent === dirname(parent)) break;
  }
}
function exactKeys(value: unknown, keys: string[]): asserts value is Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value) ||
      Object.keys(value).sort().join() !== [...keys].sort().join()) throw new Error("Invalid build declaration keys");
}
export function parseWebServerBuildConfig(value: unknown): WebServerBuildConfig {
  exactKeys(value, ["format", "web", "rust"]);
  if (value.format !== 1) throw new Error("Unsupported Web/Server build declaration");
  exactKeys(value.web, ["directory", "script", "dist"]);
  exactKeys(value.rust, ["manifest", "package", "binary", "source_revision_env"]);
  for (const [key, entry] of Object.entries(value.web)) {
    if (typeof entry !== "string" || !entry || (key === "script" && !/^[a-zA-Z0-9:_-]+$/.test(entry))) throw new Error(`Invalid web.${key}`);
  }
  for (const [key, entry] of Object.entries(value.rust)) {
    if (typeof entry !== "string" || !entry || (key !== "manifest" && !/^[a-zA-Z0-9_-]+$/.test(entry))) throw new Error(`Invalid rust.${key}`);
  }
  if (!canonicalPath(value.web.directory, true) || !canonicalPath(value.web.dist) || !canonicalPath(value.rust.manifest)) throw new Error("Build paths must be canonical relative paths");
  return value as unknown as WebServerBuildConfig;
}

/** Validate the bytes exposed by the actual executable, not merely a sibling manifest. */
export function verifyEmbeddedAssets(manifestBytes: string, directory: string): { sha256: string; files: number } {
  if (manifestBytes.endsWith("\n")) manifestBytes = manifestBytes.slice(0, -1);
  const manifest = JSON.parse(manifestBytes) as { format?: string; files?: AssetFile[] };
  exactKeys(manifest, ["format", "files"]);
  if (manifest.format !== "web-assets-v1" || !Array.isArray(manifest.files) || !manifest.files.length) throw new Error("Invalid compiled Web inventory");
  if (JSON.stringify(manifest) !== manifestBytes) throw new Error("Compiled Web inventory is not canonical JSON");
  const root = realpathSync(directory);
  const actual = new Map<string, { size: number; sha256: string }>();
  const walk = (path: string): void => {
    const meta = lstatSync(path);
    if (meta.isSymbolicLink()) throw new Error(`Linked Web input: ${path}`);
    if (meta.isDirectory()) {
      for (const name of readdirSync(path).sort()) walk(resolve(path, name));
    } else if (meta.isFile() && meta.nlink === 1) {
      actual.set(relative(root, path).split(sep).join("/"), { size: meta.size, sha256: createHash("sha256").update(readFileSync(path)).digest("hex") });
    } else throw new Error(`Unsafe Web input: ${path}`);
  };
  if (lstatSync(directory).isSymbolicLink()) throw new Error("Linked Web root");
  walk(root);
  let previous = "";
  for (const file of manifest.files) {
    exactKeys(file, ["path", "content_type", "size", "sha256"]);
    if (typeof file.path !== "string" || file.path <= previous || typeof file.content_type !== "string" ||
        !Number.isSafeInteger(file.size) || file.size < 0 || !/^[a-f0-9]{64}$/.test(file.sha256)) throw new Error("Invalid compiled asset entry");
    previous = file.path;
    const expected = actual.get(file.path);
    if (!expected || expected.size !== file.size || expected.sha256 !== file.sha256) throw new Error(`Executable Web bytes differ from build output: ${file.path}`);
    actual.delete(file.path);
  }
  if (actual.size) throw new Error(`Executable omitted Web output: ${[...actual.keys()].join(", ")}`);
  return { sha256: createHash("sha256").update(manifestBytes).digest("hex"), files: manifest.files.length };
}

function run(command: string, args: string[], cwd: string, environment: NodeJS.ProcessEnv, capture = false): string {
  const result = spawnSync(command, args, { cwd, env: environment, encoding: "utf8", maxBuffer: 64 * 1024 * 1024, stdio: capture ? ["ignore", "pipe", "inherit"] : ["inherit", 2, "inherit"] });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal})`);
  return result.stdout ?? "";
}

/** Shared acceptance hook for packagers that already verified an isolated source archive. */
export function verifyWebServerBinary(binary: string, directory: string): { sha256: string; files: number } {
  const executable = realpathSync(binary);
  const inventory = run(executable, ["web-assets"], dirname(executable), process.env, true);
  return verifyEmbeddedAssets(inventory, directory);
}

/** One ordered build contract shared by every embedded Web consumer. */
export function requireServerBuildPlatform(): void {
  const report = process.report.getReport() as { header: { glibcVersionRuntime?: string } };
  if (process.platform !== "linux" || process.arch !== "x64" || !report.header.glibcVersionRuntime) throw new Error("xcss server build inputs require Linux x86_64 with glibc");
}

export function buildWebServer(configFile: string, options: WebServerBuildOptions): string | undefined {
  requireServerBuildPlatform();
  if (process.versions.node !== WEB_TOOLCHAIN.node) throw new Error(`Node must be exactly ${WEB_TOOLCHAIN.node}`);
  if (options.webOnly && options.rustOnly) throw new Error("web-only and rust-only are mutually exclusive");
  const configPath = realpathSync(configFile);
  const root = dirname(configPath);
  const config = parseWebServerBuildConfig(JSON.parse(readFileSync(configPath, "utf8")));
  const web = within(root, config.web.directory);
  const manifest = within(root, config.rust.manifest);
  const defaultDist = within(web, config.web.dist);
  const dist = options.dist === undefined ? defaultDist : resolve(options.dist);
  assertWebOutput(root, web, defaultDist, dist);
  const environment: NodeJS.ProcessEnv = { ...process.env, XCSS_WEB_DIST: dist };
  const source = spawnSync("git", ["rev-parse", "--show-toplevel"], { cwd: root, env: environment, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
  const checkout = source.status === 0 && realpathSync(source.stdout.trim()) === root;
  const outputRelative = relative(root, dist);
  if (checkout && outputRelative !== ".." && !outputRelative.startsWith(`..${sep}`) &&
      run("git", ["ls-files", "-z", "--", outputRelative], root, environment, true)) throw new Error("Web output contains tracked source files");
  if (options.mode === "release") {
    if (!checkout) throw new Error("Release builds require their own Git checkout; verified archive packagers use the acceptance hook");
    const revision = run("git", ["rev-parse", "HEAD"], root, environment, true).trim();
    if (!/^[0-9a-f]{40}$/.test(revision) || (options.sourceRevision !== undefined && options.sourceRevision !== revision)) throw new Error("Release source revision differs from checked-out HEAD");
    if (run("git", ["status", "--porcelain=v1", "--untracked-files=all"], root, environment, true).trim()) throw new Error("Release builds require clean source");
    environment[config.rust.source_revision_env] = revision;
  } else {
    if (options.sourceRevision && options.sourceRevision !== "unbound") throw new Error("Development builds must be unbound");
    environment[config.rust.source_revision_env] = "unbound";
  }
  if (!options.rustOnly) {
    if (!options.noInstall) run("npm", ["ci", "--ignore-scripts", "--no-audit", "--no-fund"], web, environment);
    run("npm", ["run", config.web.script], web, environment);
  }
  if (options.webOnly) return undefined;
  const cargoArgs = options.cargoArgs ?? [];
  if (cargoArgs.some(arg => ["--manifest-path", "--package", "-p", "--target", "--message-format", "--config"].includes(arg) || /^--(?:manifest-path|package|target|message-format|config)=/.test(arg))) throw new Error("Cargo arguments may not override the declared build contract");
  const args = ["build", "--locked", "--manifest-path", manifest, "--package", config.rust.package, "--bin", config.rust.binary,
    "--target", "x86_64-unknown-linux-gnu", "--message-format=json-render-diagnostics", ...(options.mode === "release" ? ["--release"] : []), ...cargoArgs];
  const output = run("cargo", args, root, environment, true);
  const candidates = new Set<string>();
  for (const line of output.split("\n")) {
    if (!line.trim()) continue;
    const message = JSON.parse(line) as { reason: string; target?: { name: string; kind: string[] }; executable?: string; message?: { rendered?: string } };
    if (message.reason === "compiler-message" && message.message?.rendered) process.stderr.write(message.message.rendered);
    if (message.reason === "compiler-artifact" && message.target?.name === config.rust.binary && message.target.kind.includes("bin") && message.executable) candidates.add(realpathSync(message.executable));
  }
  if (candidates.size !== 1) throw new Error(`Cargo reported ${candidates.size} executable candidates`);
  const binary = [...candidates][0]!;
  const verified = verifyWebServerBinary(binary, dist);
  process.stderr.write(`Embedded Web verified: ${verified.files} files, SHA-256 ${verified.sha256}\n`);
  return binary;
}
