import { createHash } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
const root = new URL("../", import.meta.url);
const source = await readFile(new URL("packages/admin-ui/src/DateRangeField.tsx", root), "utf8");
const assets = {
  "DateRangeField.tsx": source.replace('from "./i18n.js"', 'from "@xcss/admin-ui/i18n"'),
  "date-range.css": await readFile(new URL("packages/admin-ui/date-range.css", root), "utf8"),
  "verify.mjs": `import { createHash } from "node:crypto";\nimport { readFile } from "node:fs/promises";\nconst manifest = JSON.parse(await readFile(new URL("snapshot.json", import.meta.url), "utf8"));\nif (manifest.origin !== "@xcss/admin-ui/date-range") throw new Error("Invalid date range snapshot origin");\nfor (const [name, digest] of Object.entries(manifest.assets)) {\n  if (createHash("sha256").update(await readFile(new URL(name, import.meta.url))).digest("hex") !== digest) throw new Error(\`Date range snapshot mismatch: \${name}\`);\n}\n`,
};
const manifest = JSON.stringify({ origin: "@xcss/admin-ui/date-range", assets: Object.fromEntries(Object.entries(assets).map(([name, text]) => [name, createHash("sha256").update(text).digest("hex")])) }, null, 2) + "\n";
for (const product of ["xsos", "xscs", "xcos", "xszs"]) {
  const directory = new URL(`../${product}/web/src/date-range/`, root);
  if (process.argv.includes("--write")) {
    await mkdir(directory, { recursive: true });
    for (const [name, text] of Object.entries({ ...assets, "snapshot.json": manifest })) await writeFile(new URL(name, directory), text);
  } else {
    for (const [name, expected] of Object.entries({ ...assets, "snapshot.json": manifest })) {
      if (await readFile(new URL(name, directory), "utf8") !== expected) throw new Error(`${product}: outdated date range snapshot ${name}`);
    }
  }
  console.log(`${product}: xcss date range snapshot ${process.argv.includes("--write") ? "updated" : "verified"}`);
}
