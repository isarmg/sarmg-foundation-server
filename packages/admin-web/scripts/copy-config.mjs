import { cp } from "node:fs/promises";

await cp(new URL("../tsconfig.json", import.meta.url), new URL("../dist/tsconfig.json", import.meta.url));
