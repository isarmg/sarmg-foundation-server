import { rm } from "node:fs/promises";

await rm(new URL("../../../dist/contracts/", import.meta.url), { force: true, recursive: true });
