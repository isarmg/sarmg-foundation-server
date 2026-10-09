// Export only reviewed changes; keep published release URLs and integrity intact.
import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("../", import.meta.url));
const digest = bytes => createHash("sha256").update(bytes).digest("hex");
const args = process.argv.slice(2);
if (args[0] !== "--baseline" || args.length < 3) {
  throw new Error("Usage: node scripts/export-consumer-web-patches.mjs --baseline <unpatched consumer package root> <consumer package root>...");
}
const baseline = resolve(args[1]);
const targets = args.slice(2).map(target => resolve(target));
const locked = JSON.parse(await readFile(join(baseline, "package-lock.json"), "utf8"));
const files = [];

function editsBetween(before, after) {
  const previous = before.toString("utf8").match(/[^\n]*\n|[^\n]+$/gu) ?? [];
  const next = after.toString("utf8").match(/[^\n]*\n|[^\n]+$/gu) ?? [];
  const lengths = Array.from({ length: previous.length + 1 }, () => new Uint32Array(next.length + 1));
  for (let left = previous.length - 1; left >= 0; left -= 1) {
    for (let right = next.length - 1; right >= 0; right -= 1) {
      lengths[left][right] = previous[left] === next[right]
        ? lengths[left + 1][right + 1] + 1
        : Math.max(lengths[left + 1][right], lengths[left][right + 1]);
    }
  }
  const edits = [];
  let left = 0, right = 0, offset = 0, active;
  const flush = () => { if (active) edits.push(active); active = undefined; };
  while (left < previous.length || right < next.length) {
    if (left < previous.length && right < next.length && previous[left] === next[right]) {
      flush();
      offset += Buffer.byteLength(previous[left]);
      left += 1; right += 1;
    } else {
      active ??= { offset, before: "", after: "" };
      if (right < next.length && (left === previous.length || lengths[left][right + 1] >= lengths[left + 1][right])) {
        active.after += next[right++];
      } else {
        active.before += previous[left];
        offset += Buffer.byteLength(previous[left++]);
      }
    }
  }
  flush();
  return edits;
}

for (const [name, path, sourcePath, baselineSha256] of [
  ["admin-shell", "dist/account.js", "src/account.tsx", "1f77c015cd08c9c8489cf2649051720d2c5f0c2c0c5eced84a99483e56ca2b05"],
  ["admin-ui", "dist/content-blocks.css", "content-blocks.css", "bd409f02eac4409e98e3113e7756c800dac8e22d06091521747738752e55a814"],
]) {
  const packageName = "@xcss/web";
  const dependency = locked.packages[`node_modules/${packageName}`];
  const publishedPath = `dist/${name}/${path.slice("dist/".length)}`;
  const before = await readFile(join(baseline, "node_modules", packageName, publishedPath));
  const after = await readFile(join(root, "dist", name, path.slice("dist/".length)));
  const source = `web/${name}/${sourcePath}`;
  const edits = editsBetween(before, after);
  const releaseUrl = `https://github.com/isarmg/xcss/releases/download/v1.0.0/xcss-web-1.0.0.tgz`;
  if (dependency.version !== "1.0.0" || dependency.resolved !== releaseUrl || digest(before) !== baselineSha256) {
    throw new Error(`Expected audited original 1.0.0 baseline and matching built output: ${packageName}`);
  }
  files.push({
    package: packageName, path: publishedPath,
    releaseUrl: dependency.resolved, releaseIntegrity: dependency.integrity,
    source, sourceSha256: digest(await readFile(join(root, source))),
    baselineSha256: digest(before), patchedSha256: digest(after), edits,
  });
}
const manifest = JSON.stringify({
  format: 1, xcssVersion: "1.0.0",
  sourceRepository: "https://github.com/isarmg/xcss",
  distribution: "Reviewed source changes applied during builds; immutable release dependencies unchanged",
  files,
}, null, 2) + "\n";
const loader = await readFile(new URL("consumer-patches/apply-xcss-patches.mjs", import.meta.url));
for (const target of targets) {
  await readFile(join(target, "package.json"));
  await mkdir(join(target, "scripts"), { recursive: true });
  await mkdir(join(target, "patches"), { recursive: true });
  await writeFile(join(target, "scripts/apply-xcss-patches.mjs"), loader);
  await writeFile(join(target, "patches/xcss.json"), manifest);
  console.log(`${target}: reviewed xcss Web patches exported`);
}
