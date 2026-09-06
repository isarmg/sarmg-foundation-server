import { copyFile } from "node:fs/promises";
for (const name of ["styles.css", "content-blocks.css"]) {
  await copyFile(new URL(`../${name}`, import.meta.url), new URL(`../dist/${name}`, import.meta.url));
}
