// Distribute a reviewed source snapshot, never replace a published npm tarball.
import { copyFile, mkdir, readFile, writeFile, unlink } from "node:fs/promises";
import { resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const source = fileURLToPath(new URL("../packages/web-fonts/", import.meta.url));
const provenance = JSON.parse(await readFile(join(source, "provenance.json"), "utf8"));
const names = [...Object.keys(provenance.assets).filter(name => !["MapleMono.woff2", "MapleMono-Italic.woff2"].includes(name)), "provenance.json"];
for (const name of names) {
  if (name === "provenance.json") continue;
  const digest = createHash("sha256").update(await readFile(join(source, name))).digest("hex");
  if (digest !== provenance.assets[name]) throw new Error(`Source font digest mismatch: ${name}`);
}
if (process.argv.length < 3) throw new Error("Pass explicit product repository roots");
for (const argument of process.argv.slice(2)) {
  const product = resolve(argument);
  await readFile(join(product, "clients/web/package.json"));
  const destination = join(product, "clients/web/fonts");
  await mkdir(destination, { recursive: true });
  // Remove only the verified, previously distributed Western faces, never user files.
  for (const name of ["MapleMono.woff2", "MapleMono-Italic.woff2"]) {
    try {
      const bytes = await readFile(join(destination, name));
      if (createHash("sha256").update(bytes).digest("hex") !== provenance.assets[name]) throw new Error(`Unexpected font file: ${name}`);
      await unlink(join(destination, name));
    } catch (error) { if (error.code !== "ENOENT") throw error; }
  }
  for (const name of names) {
    const target = join(destination, name);
    await mkdir(resolve(target, ".."), { recursive: true });
    await copyFile(join(source, name), target);
  }
  const snapshot = structuredClone(provenance);
  delete snapshot.assets["MapleMono.woff2"];
  delete snapshot.assets["MapleMono-Italic.woff2"];
  snapshot.distribution = "Reviewed Server Foundation font source snapshot; published npm dependencies remain unchanged";
  await writeFile(join(destination, "provenance.json"), JSON.stringify(snapshot, null, 2) + "\n");
  await copyFile(new URL("../packages/web-fonts/scripts/verify-snapshot.mjs", import.meta.url), join(destination, "verify.mjs"));
  console.log(`${product}: synchronized ${names.length} reviewed font files`);
}
