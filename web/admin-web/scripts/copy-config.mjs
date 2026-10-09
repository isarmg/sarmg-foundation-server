import { cp } from "node:fs/promises";

await cp(new URL("../tsconfig.json", import.meta.url), new URL("../../../dist/admin-web/tsconfig.json", import.meta.url));
