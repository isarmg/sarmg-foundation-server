import assert from "node:assert/strict";
import { access, readFile } from "node:fs/promises";
import test from "node:test";

import { semanticTokens, tokens } from "../dist/index.js";

const packageRoot = new URL("../", import.meta.url);

function cssDeclarations(css, expectedSelector) {
  const match = css.match(/^([^{}]+)\{([^{}]*)\}\s*$/s);
  assert.ok(match, "CSS must contain exactly one flat selector block");
  assert.equal(match[1].trim(), expectedSelector);

  const declarations = {};
  for (const raw of match[2].split(";")) {
    const declaration = raw.trim();
    if (!declaration) continue;
    const parts = declaration.match(/^(--sarmg-[a-z0-9-]+):\s*(.+)$/);
    assert.ok(parts, `invalid token declaration: ${declaration}`);
    assert.equal(declarations[parts[1]], undefined, `duplicate token ${parts[1]}`);
    declarations[parts[1]] = parts[2];
  }
  return declarations;
}

function expectedPrimitiveVariables() {
  const variables = {};
  for (const [name, value] of Object.entries(tokens.color)) {
    const cssName = name.replace(/([a-z])([0-9])/g, "$1-$2");
    variables[`--sarmg-${cssName}`] = value;
  }
  for (const [name, value] of Object.entries(tokens.space)) {
    variables[`--sarmg-space-${name}`] = value;
  }
  for (const [name, value] of Object.entries(tokens.radius)) {
    variables[`--sarmg-radius-${name}`] = value;
  }
  for (const [name, value] of Object.entries(tokens.typography)) {
    const cssName = name.replace(/([a-z])([A-Z])/g, "$1-$2").toLowerCase();
    variables[`--sarmg-${cssName}`] = value;
  }
  return variables;
}

const semanticCssNames = {
  actionPrimary: "--sarmg-action-primary",
  bgPage: "--sarmg-bg-page",
  bgPanel: "--sarmg-bg-panel",
  textPrimary: "--sarmg-text-primary",
  textDanger: "--sarmg-text-danger",
};

function resolveCustomProperties(declarations) {
  const resolved = {};
  const resolving = new Set();

  function resolve(name) {
    assert.ok(Object.hasOwn(declarations, name), `undefined CSS token ${name}`);
    if (Object.hasOwn(resolved, name)) return resolved[name];
    assert.ok(!resolving.has(name), `cyclic CSS token reference at ${name}`);
    resolving.add(name);
    const raw = declarations[name];
    const reference = raw.match(/^var\((--sarmg-[a-z0-9-]+)\)$/);
    const value = reference ? resolve(reference[1]) : raw;
    resolving.delete(name);
    resolved[name] = value;
    return value;
  }

  for (const name of Object.keys(declarations)) resolve(name);
  return resolved;
}

test("clean build contains no stale output", async () => {
  await assert.rejects(access(new URL("../dist/stale-build-output.txt", import.meta.url)));
});

test("TypeScript exports and light/dark CSS remain exactly aligned", async () => {
  const [lightSource, darkSource, lightBuilt, darkBuilt] = await Promise.all([
    readFile(new URL("tokens.css", packageRoot), "utf8"),
    readFile(new URL("tokens.dark.css", packageRoot), "utf8"),
    readFile(new URL("dist/tokens.css", packageRoot), "utf8"),
    readFile(new URL("dist/tokens.dark.css", packageRoot), "utf8"),
  ]);
  assert.equal(lightBuilt, lightSource, "published light CSS differs from its source");
  assert.equal(darkBuilt, darkSource, "published dark CSS differs from its source");

  const light = cssDeclarations(lightBuilt, ":root");
  const darkOverrides = cssDeclarations(darkBuilt, ':root[data-theme="dark"]');
  assert.deepEqual(
    Object.fromEntries(
      Object.entries(light).filter(([name]) => name.startsWith("--sarmg-") &&
        !Object.values(semanticCssNames).includes(name)),
    ),
    expectedPrimitiveVariables(),
  );

  const effectiveLight = resolveCustomProperties(light);
  const effectiveDark = resolveCustomProperties({ ...light, ...darkOverrides });
  const expectedDarkOverrides = {};
  for (const [key, cssName] of Object.entries(semanticCssNames)) {
    assert.equal(effectiveLight[cssName], semanticTokens.light[key]);
    assert.equal(effectiveDark[cssName], semanticTokens.dark[key]);
    if (semanticTokens.light[key] !== semanticTokens.dark[key]) {
      expectedDarkOverrides[cssName] = semanticTokens.dark[key];
    }
  }
  assert.deepEqual(darkOverrides, expectedDarkOverrides);
});

test("package exports resolve only to clean, published artifacts", async () => {
  const manifest = JSON.parse(
    await readFile(new URL("package.json", packageRoot), "utf8"),
  );
  assert.deepEqual(manifest.files, ["dist"]);
  assert.deepEqual(Object.keys(manifest.exports).sort(), [
    ".",
    "./accessibility.css",
    "./reset.css",
    "./tokens.css",
    "./tokens.dark.css",
  ]);

  const targets = [manifest.main, manifest.types];
  for (const value of Object.values(manifest.exports)) {
    if (typeof value === "string") targets.push(value);
    else targets.push(...Object.values(value));
  }
  for (const target of new Set(targets)) {
    assert.match(target, /^\.\/dist\/[a-z0-9./-]+$/);
    await access(new URL(target.slice(2), packageRoot));
  }

  const declaration = await readFile(new URL("dist/index.d.ts", packageRoot), "utf8");
  assert.match(declaration, /export declare const tokens:/);
  assert.match(declaration, /export declare const semanticTokens:/);
});

test("reset and accessibility baselines are scoped, copied exactly, and opt-in", async () => {
  for (const name of ["reset.css", "accessibility.css"]) {
    const [source, built] = await Promise.all([
      readFile(new URL(name, packageRoot), "utf8"),
      readFile(new URL(`dist/${name}`, packageRoot), "utf8"),
    ]);
    assert.equal(built, source, `published ${name} differs from its source`);
    assert.match(source, /\[data-sarmg-scope\]/);
    assert.doesNotMatch(source, /(^|[},\n]\s*)(html|body|\*)\s*[{,]/m);
  }

  const accessibility = await readFile(new URL("accessibility.css", packageRoot), "utf8");
  assert.match(accessibility, /:focus-visible/);
  assert.match(accessibility, /prefers-reduced-motion:\s*reduce/);
  assert.match(accessibility, /forced-colors:\s*active/);
});
