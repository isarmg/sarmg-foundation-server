import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { applyXcssPatches } from "./apply-xcss-patches.mjs";

const digest = text => createHash("sha256").update(text).digest("hex");

function fixture() {
  const root = mkdtempSync(join(tmpdir(), "xcss-consumer-patches-"));
  const lock = { packages: {} };
  const files = [];
  for (const [name, path] of [["admin-shell", "dist/admin-shell/account.js"], ["admin-ui", "dist/admin-ui/content-blocks.css"]]) {
    const packageName = "@xcss/web";
    const packageRoot = `node_modules/${packageName}`;
    const before = `original ${name}\n`;
    const after = `reviewed ${name}\n`;
    const releaseUrl = `https://github.com/isarmg/xcss/releases/download/v1.0.0/xcss-web-1.0.0.tgz`;
    const releaseIntegrity = "sha512-review-fixture";
    mkdirSync(join(root, packageRoot, "dist", name), { recursive: true });
    writeFileSync(join(root, packageRoot, "package.json"), JSON.stringify({ name: packageName, version: "1.0.0" }));
    writeFileSync(join(root, packageRoot, path), before);
    lock.packages[packageRoot] = { version: "1.0.0", resolved: releaseUrl, integrity: releaseIntegrity };
    files.push({ package: packageName, path, releaseUrl, releaseIntegrity,
      baselineSha256: digest(before), patchedSha256: digest(after), edits: [{ offset: 0, before, after }] });
  }
  mkdirSync(join(root, "patches"));
  writeFileSync(join(root, "package-lock.json"), JSON.stringify(lock));
  writeFileSync(join(root, "patches/xcss.json"), JSON.stringify({ format: 1, xcssVersion: "1.0.0", files }));
  return { root, files, dispose: () => rmSync(root, { recursive: true, force: true }) };
}

test("reviewed patches apply to a restored release and are idempotent", () => {
  const input = fixture();
  try {
    applyXcssPatches(input.root);
    for (const file of input.files) {
      assert.equal(digest(readFileSync(join(input.root, "node_modules", file.package, file.path))), file.patchedSha256);
    }
    applyXcssPatches(input.root);
  } finally { input.dispose(); }
});

test("unknown second baseline leaves the valid first file unchanged", () => {
  const input = fixture();
  try {
    const first = input.files[0];
    const second = input.files[1];
    writeFileSync(join(input.root, "node_modules", second.package, second.path), "local modification\n");
    assert.throws(() => applyXcssPatches(input.root), /Unknown xcss patch baseline/);
    assert.equal(digest(readFileSync(join(input.root, "node_modules", first.package, first.path))), first.baselineSha256);
    assert.equal(readFileSync(join(input.root, "node_modules", second.package, second.path), "utf8"), "local modification\n");
  } finally { input.dispose(); }
});

test("upgraded dependency identities fail before any source edits", () => {
  const input = fixture();
  try {
    const second = input.files[1];
    writeFileSync(join(input.root, "node_modules", second.package, "package.json"), JSON.stringify({ name: second.package, version: "1.0.1" }));
    assert.throws(() => applyXcssPatches(input.root), /dependency identity changed/);
    const first = input.files[0];
    assert.equal(digest(readFileSync(join(input.root, "node_modules", first.package, first.path))), first.baselineSha256);
  } finally { input.dispose(); }
});

test("a corrupted patch output does not write the previously validated file", () => {
  const input = fixture();
  try {
    input.files[1].patchedSha256 = "0".repeat(64);
    writeFileSync(join(input.root, "patches/xcss.json"), JSON.stringify({ format: 1, xcssVersion: "1.0.0", files: input.files }));
    assert.throws(() => applyXcssPatches(input.root), /output hash mismatch/);
    const first = input.files[0];
    assert.equal(digest(readFileSync(join(input.root, "node_modules", first.package, first.path))), first.baselineSha256);
  } finally { input.dispose(); }
});

test("linked installed files are rejected without following the link", () => {
  const input = fixture();
  const outside = mkdtempSync(join(tmpdir(), "xcss-unrelated-patch-input-"));
  try {
    const file = input.files[0];
    const target = join(input.root, "node_modules", file.package, file.path);
    const external = join(outside, "account.js");
    writeFileSync(external, file.edits[0].before);
    rmSync(target);
    symlinkSync(external, target);
    assert.throws(() => applyXcssPatches(input.root), /must be a real file/);
    assert.equal(readFileSync(external, "utf8"), file.edits[0].before);
  } finally { input.dispose(); rmSync(outside, { recursive: true, force: true }); }
});
