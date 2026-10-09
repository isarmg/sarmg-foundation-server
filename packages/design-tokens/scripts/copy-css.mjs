import { cp } from "node:fs/promises";

for (const name of ["accessibility.css", "reset.css", "tokens.css", "tokens.dark.css"]) {
  await cp(new URL(`../${name}`, import.meta.url), new URL(`../dist/${name}`, import.meta.url));
}
