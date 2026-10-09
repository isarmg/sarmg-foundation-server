import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";

const provenance = JSON.parse(await readFile(new URL("../provenance.json", import.meta.url), "utf8"));
for (const [name, expected] of Object.entries(provenance.assets)) {
  const bytes = await readFile(new URL(`../${name}`, import.meta.url));
  const actual = createHash("sha256").update(bytes).digest("hex");
  if (actual !== expected) throw new Error(`${name} provenance mismatch`);
}
const css = await readFile(new URL("../fonts.css", import.meta.url), "utf8");
const faces = [...css.matchAll(/@font-face\{([^}]+)\}/gu)];
assert.ok(faces.length > 2);
for (const [, face] of faces) {
  assert.ok(face.includes("font-display:block;"), "Every font is prepared before the UI is displayed");
}

const startup = JSON.parse(await readFile(new URL("../startup.json", import.meta.url), "utf8"));
assert.equal(startup.policy, "all-fonts-before-ui");
for (const [name, expected] of Object.entries(startup.assets)) {
  assert.equal(createHash("sha256").update(await readFile(new URL(`../${name}`, import.meta.url))).digest("hex"), expected, name);
}
