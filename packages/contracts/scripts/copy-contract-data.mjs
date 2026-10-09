import { cp, mkdir, readdir } from "node:fs/promises";

for (const directory of ["schemas", "fixtures"]) {
  const source = new URL(`../${directory}/`, import.meta.url);
  const destination = new URL(`../dist/${directory}/`, import.meta.url);
  await mkdir(destination, { recursive: true });
  for (const name of (await readdir(source)).filter((name) => name.endsWith(".json")).sort()) {
    await cp(new URL(name, source), new URL(name, destination));
  }
}
