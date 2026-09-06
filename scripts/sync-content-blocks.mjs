// Reviewed source distribution; does not mutate any published Foundation package.
import { readFile, writeFile, mkdir, copyFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { createHash } from "node:crypto";
const source = new URL("../packages/admin-ui/content-blocks.css", import.meta.url);
const css = await readFile(source);
if (process.argv.length < 3) throw new Error("Pass explicit product repository roots");
for (const argument of process.argv.slice(2)) {
  const root = resolve(argument, "clients/web");
  await readFile(join(root, "package.json"));
  const destination = join(root, "appearance");
  await mkdir(destination, { recursive: true });
  await copyFile(source, join(destination, "content-blocks.css"));
  await copyFile(new URL("../packages/admin-ui/scripts/verify-content-blocks.mjs", import.meta.url), join(destination, "verify.mjs"));
  await writeFile(join(destination, "provenance.json"), JSON.stringify({
    appearance: "content-blocks", default: true, consumerOverride: true,
    source: "sarmg-foundation-server/packages/admin-ui/content-blocks.css",
    origin: "union-rust/web/vendor/sarmg-design (Apache-2.0, sarmg)",
    license: "Apache-2.0",
    distribution: "Reviewed source snapshot; published npm dependencies remain unchanged",
    sha256: createHash("sha256").update(css).digest("hex"),
  }, null, 2) + "\n");
  console.log(`${argument}: synchronized default content-block appearance (consumer-overridable)`);
}
