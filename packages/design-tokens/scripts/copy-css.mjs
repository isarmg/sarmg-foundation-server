import { cp } from "node:fs/promises";

await cp(new URL("../tokens.css", import.meta.url), new URL("../dist/tokens.css", import.meta.url));
await cp(new URL("../tokens.dark.css", import.meta.url), new URL("../dist/tokens.dark.css", import.meta.url));
