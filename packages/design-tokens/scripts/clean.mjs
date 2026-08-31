import { rm } from "node:fs/promises";

// Deliberately use a package-relative URL: this command must never accept a
// caller-controlled deletion target.
await rm(new URL("../dist/", import.meta.url), { recursive: true, force: true });
