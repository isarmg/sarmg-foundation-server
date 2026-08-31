import { cp, mkdir, readdir } from "node:fs/promises";

await mkdir(new URL("../dist/schemas/", import.meta.url), { recursive: true });
const source = new URL("../schemas/", import.meta.url);
const destination = new URL("../dist/schemas/", import.meta.url);
for (const name of (await readdir(source)).filter((name) => name.endsWith(".schema.json")).sort()) {
  await cp(new URL(name, source), new URL(name, destination));
}
