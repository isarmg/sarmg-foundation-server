import { copyFile, mkdir, readFile, rm, writeFile } from "node:fs/promises";
await import("./verify.mjs");
const provenance = JSON.parse(await readFile(new URL("../provenance.json", import.meta.url), "utf8"));
await rm(new URL("../dist", import.meta.url), { recursive: true, force: true });
for (const name of [...Object.keys(provenance.assets), "provenance.json"]) {
  const destination = new URL(`../dist/${name}`, import.meta.url);
  await mkdir(new URL(".", destination), { recursive: true });
  await copyFile(new URL(`../${name}`, import.meta.url), destination);
}
await writeFile(new URL("../dist/index.js", import.meta.url), `export const mapleFontProvenance = Object.freeze(${JSON.stringify({ sourceCommit: provenance.source_commit, cjkRelease: "v7.9", ligatures: false })});\n`);
await writeFile(new URL("../dist/index.d.ts", import.meta.url), `export declare const mapleFontProvenance: Readonly<{ sourceCommit: '${provenance.source_commit}'; cjkRelease: 'v7.9'; ligatures: false }>;\n`);
