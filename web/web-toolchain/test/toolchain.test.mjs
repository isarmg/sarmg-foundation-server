import test from"node:test";import assert from"node:assert/strict";import{assertXcssWebToolchain}from"../../../dist/web-toolchain/index.js";test("rejects drift",()=>assert.throws(()=>assertXcssWebToolchain({engines:{node:">=26.7.0 <27"},dependencies:{react:"latest"}},"26.7.0")));
import { createXcssReactViteConfig } from "../../../dist/web-toolchain/vite.js";
import { createXcssNativeModuleViteConfig } from "../../../dist/web-toolchain/native.js";
test("native ESM uses relative assets, no React and the platform hard budget", () => {
  const config = createXcssNativeModuleViteConfig({ entry: "web/platform.js", outDir: "web/dist" });
  assert.equal(config.base, "./");
  assert.equal(config.build.sourcemap, false);
  assert.equal(config.build.assetsInlineLimit, 0);
  assert.equal(config.build.rollupOptions.preserveEntrySignatures, "strict");
  assert.equal(config.build.rollupOptions.output.format, "es");
  assert.equal(config.build.rollupOptions.output.entryFileNames, "platform.js");
  assert.deepEqual(config.plugins.map(plugin => plugin.name), ["xcss-asset-budget"]);
  const context = { error(message) { throw new Error(message); } };
  assert.throws(() => config.plugins[0].generateBundle.call(context, {}, { "oversize.js": { type: "chunk", code: "x".repeat(256 * 1024 + 1) } }), /exceeds/);
  assert.doesNotThrow(() => config.plugins[0].generateBundle.call(context, {}, { "font.woff2": { type: "asset", source: new Uint8Array(256 * 1024) } }));
});
test("asset sizes and source maps are hard release gates, not warnings", () => {
  const config = createXcssReactViteConfig({ maxAssetBytes: 1024 });
  const budget = config.plugins.find(plugin => plugin?.name === "xcss-asset-budget");
  const context = { error(message) { throw new Error(message); } };
  assert.throws(() => budget.generateBundle.call(context, {}, { "image.png": { type: "asset", source: new Uint8Array(1025) } }), /exceeds/);
  assert.throws(() => budget.generateBundle.call(context, {}, { "main.js.map": { type: "asset", source: "{}" } }), /source maps/);
  assert.doesNotThrow(() => budget.generateBundle.call(context, {}, { "main.js": { type: "chunk", code: "hello" } }));
  for (const maxAssetBytes of [0, -1, 1.5, Infinity, 67108865]) assert.throws(() => createXcssReactViteConfig({ maxAssetBytes }));
  assert.equal(config.build.sourcemap, false);
  assert.deepEqual(config.resolve.dedupe, ["react", "react-dom"]);
});
