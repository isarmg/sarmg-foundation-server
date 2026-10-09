import{copyFile}from"node:fs/promises";await copyFile(new URL("../tsconfig.json",import.meta.url),new URL("../dist/tsconfig.json",import.meta.url));
