import { rm } from "node:fs/promises";

await rm(new URL("../../../dist/http-client/", import.meta.url), { force: true, recursive: true });
