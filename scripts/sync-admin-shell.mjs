// Distribute reviewed compiled sources without changing immutable npm packages.
import { readFile, readdir, mkdir, copyFile, writeFile, unlink, access } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createHash } from "node:crypto";
const source = new URL("../packages/admin-shell/dist/", import.meta.url);
const names = (await readdir(source)).filter(name => name.endsWith(".js") || name.endsWith(".d.ts")).sort();
// Ship reviewed UI/i18n sources with the shell; published npm assets stay immutable.
const uiSource = new URL("../packages/admin-ui/dist/", import.meta.url);
names.push("ui.js", "ui.d.ts", "i18n.js", "i18n.d.ts");
const bytes = async name => {
  const url = name.startsWith("ui.") ? new URL(name.replace(/^ui\./, "index."), uiSource)
    : name.startsWith("i18n.") ? new URL(name, uiSource) : new URL(name, source);
  return (await readFile(url, "utf8")).replaceAll('"@sarmg/admin-ui/i18n"', '"./i18n.js"');
};
if (!names.includes("index.js") || process.argv.length < 3) throw new Error("Build admin-shell and pass explicit consumer roots");
const assets = {};
for (const name of names) assets[name] = createHash("sha256").update(await bytes(name)).digest("hex");
for (const product of process.argv.slice(2)) {
  let root = resolve(product, "clients/web");
  try { await access(join(root, "package.json")); } catch { root = resolve(product, "web"); }
  const manifest = JSON.parse(await readFile(join(root, "package.json"), "utf8"));
  const selected = manifest.dependencies?.react ? names : names.filter(name => /^(?:native-workspace|workspace-config|i18n)\./.test(name));
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
  for (const name of selected) await writeFile(join(target, name), await bytes(name));
  await writeFile(join(target, "provenance.json"), JSON.stringify({ source: "sarmg-foundation-server/packages/admin-shell", additionalSources: ["sarmg-foundation-server/packages/admin-ui"], license: "Apache-2.0", distribution: "Reviewed source snapshot; immutable npm dependencies unchanged", assets: Object.fromEntries(selected.map(name=>[name,assets[name]])) }, null, 2) + "\n");
  await copyFile(new URL("./verify-shell-snapshot.mjs", import.meta.url), join(target, "verify.mjs"));
  console.log(`${product}: shared Shell snapshot synchronized`);
}
