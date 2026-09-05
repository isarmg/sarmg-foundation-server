import type { Plugin } from "vite";

export function assetBudget(maximum: number): Plugin {
  if (!Number.isSafeInteger(maximum) || maximum < 1 || maximum > 64 * 1024 * 1024) throw new TypeError("maxAssetBytes must be an integer in 1..67108864");
  return {
    name: "sarmg-asset-budget", enforce: "post",
    generateBundle(_options, bundle) {
      for (const [name, asset] of Object.entries(bundle)) {
        const source = asset.type === "chunk" ? asset.code : asset.source;
        const size = typeof source === "string" ? new TextEncoder().encode(source).byteLength : source.byteLength;
        if (size > maximum) this.error(`Asset ${name} exceeds the ${maximum}-byte Foundation budget (${size} bytes)`);
        if (name.endsWith(".map")) this.error("Foundation web builds must not publish source maps");
      }
    },
  };
}
