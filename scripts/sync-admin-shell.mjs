// Distribute reviewed compiled sources without changing immutable npm packages.
import { readFile, readdir, mkdir, copyFile, writeFile, unlink } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createHash } from "node:crypto";
const source = new URL("../packages/admin-shell/dist/", import.meta.url);
const names = (await readdir(source)).filter(name => name.endsWith(".js") || name.endsWith(".d.ts")).sort();
if (!names.includes("index.js") || process.argv.length < 3) throw new Error("Build admin-shell and pass explicit consumer roots");
const assets = {};
for (const name of names) assets[name] = createHash("sha256").update(await readFile(new URL(name, source))).digest("hex");
for (const product of process.argv.slice(2)) {
  const root = resolve(product, "clients/web"); const manifest = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
  const selected = manifest.dependencies?.react ? names : names.filter(name => /^(?:native-workspace|workspace-config)\./.test(name));
  const target = join(root, "shell"); await mkdir(target, { recursive: true });
  let previous = {};
  try { previous = JSON.parse(await readFile(join(target,"provenance.json"),"utf8")).assets; } catch(error) { if(error.code!=="ENOENT") throw error; }
  for (const [name,hash] of Object.entries(previous)) {
    if (selected.includes(name)) continue;
    if (!/^[a-z-]+\.(?:js|d\.ts)$/.test(name)) throw new Error("Unsafe snapshot asset");
    const bytes = await readFile(join(target,name));
    if (createHash("sha256").update(bytes).digest("hex") !== hash) throw new Error("Consumer modified snapshot: " + name);
    await unlink(join(target,name));
  }
  for (const name of selected) await copyFile(new URL(name, source), join(target, name));
  await writeFile(join(target, "provenance.json"), JSON.stringify({ source: "sarmg-foundation-server/packages/admin-shell", license: "Apache-2.0", distribution: "Reviewed source snapshot; immutable npm dependencies unchanged", assets: Object.fromEntries(selected.map(name=>[name,assets[name]])) }, null, 2) + "\n");
  await copyFile(new URL("./verify-shell-snapshot.mjs", import.meta.url), join(target, "verify.mjs"));
  console.log(`${product}: shared Shell snapshot synchronized`);
}
