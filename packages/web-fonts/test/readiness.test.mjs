import assert from "node:assert/strict";
import test from "node:test";

function deferred() {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
}

test("startup waits for unused shards and decoding, and shares a single preparation", async () => {
  const originalDocument = globalThis.document;
  const shard = deferred(), decoded = deferred();
  const calls = [];
  const faces = ["Sarmg Maple Bootstrap", "Sarmg Maple", "Sarmg Maple"].map((family, index) => ({
    family, status: "unloaded",
    async load() { calls.push(index); if (index === 2) await shard.promise; this.status = "loaded"; return this; },
  }));
  globalThis.document = { documentElement: { dataset: {} }, fonts: Object.assign(faces, { ready: decoded.promise }) };
  try {
    const { prepareApplicationFonts, startAfterFonts } = await import("../ready.js?success");
    let started = false;
    const first = prepareApplicationFonts();
    assert.equal(prepareApplicationFonts(), first);
    const startup = startAfterFonts(() => { started = true; });
    await Promise.resolve();
    assert.deepEqual(calls, [0, 1, 2]);
    assert.equal(started, false);
    assert.equal(document.documentElement.dataset.xcssFonts, "pending");
    shard.resolve();
    await Promise.resolve();
    assert.equal(started, false, "downloading every shard is insufficient until decoding settles");
    decoded.resolve();
    await startup;
    assert.equal(started, true);
    assert.equal(document.documentElement.dataset.xcssFonts, "ready");
    await prepareApplicationFonts();
    assert.deepEqual(calls, [0, 1, 2], "navigation must not reload faces");
  } finally { globalThis.document = originalDocument; }
});

test("a failed font keeps the background and never starts a fallback UI", async () => {
  const originalDocument = globalThis.document, originalError = console.error;
  const failures = [];
  const face = { family: "Sarmg Maple", status: "error", load: () => Promise.reject(new Error("failed font")) };
  globalThis.document = { documentElement: { dataset: {} }, fonts: Object.assign([face], { ready: Promise.resolve() }) };
  console.error = (...args) => failures.push(args);
  try {
    const { startAfterFonts } = await import("../ready.js?failure");
    let started = false;
    await startAfterFonts(() => { started = true; });
    assert.equal(started, false);
    assert.equal(document.documentElement.dataset.xcssFonts, "pending");
    assert.equal(failures.length, 1);
  } finally { globalThis.document = originalDocument; console.error = originalError; }
});

test("missing font styles cannot reveal the page", async () => {
  const originalDocument = globalThis.document;
  globalThis.document = { documentElement: { dataset: {} }, fonts: [] };
  try {
    const { prepareApplicationFonts } = await import("../ready.js?missing");
    await assert.rejects(prepareApplicationFonts(), /styles must load/);
    assert.equal(document.documentElement.dataset.xcssFonts, "pending");
  } finally { globalThis.document = originalDocument; }
});
