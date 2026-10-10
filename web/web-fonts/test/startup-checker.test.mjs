import assert from "node:assert/strict";
import test from "node:test";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fontSourceHandler, parseFontStartupOptions } from "../../../scripts/check-font-startup.mjs";

const fonts = resolve(import.meta.dirname, "..");

test("the startup checker accepts only explicit sources or consumer-hosted pages", () => {
  const source = parseFontStartupOptions(["--source", fonts]);
  assert.equal(source.source, fonts);
  assert.deepEqual(source.urls, []);
  assert.equal(source.colorScheme, "light");
  const urls = ["http://127.0.0.1:4173/console/", "http://127.0.0.1:4173/sign-in"];
  const pages = parseFontStartupOptions(["--url", urls[0], "--url", urls[1], "--color-scheme", "dark", "--storage-state", "session.json"]);
  assert.deepEqual(pages.urls, urls);
  assert.equal(pages.source, undefined);
  assert.equal(pages.colorScheme, "dark");
  assert.equal(pages.storageState, "session.json");
  assert.throws(() => parseFontStartupOptions([]), /either --source/);
  assert.throws(() => parseFontStartupOptions(["--source", fonts, "--url", urls[0]]), /either --source/);
  assert.throws(() => parseFontStartupOptions(["--source", fonts, "--color-scheme", "automatic"]), /light or dark/);
  assert.throws(() => parseFontStartupOptions(["--url", "file:///tmp/page.html"]), /HTTP or HTTPS/);
});

async function request(url) {
  const response = { statusCode: 200, headers: {}, setHeader(name, value) { this.headers[name] = value; }, end(body) { this.body = body; } };
  await fontSourceHandler(fonts)({ url }, response);
  return response;
}

test("the standalone source fixture serves its actual startup and font assets", async () => {
  const page = await request("/font-startup");
  assert.equal(page.statusCode, 200);
  assert.equal(page.headers["content-type"], "text/html; charset=utf-8");
  assert.ok(page.body.includes(await readFile(resolve(fonts, "boot.css"), "utf8")));
  assert.match(page.body, /data-xcss-fonts="pending"/u);
  assert.match(page.body, /startAfterFonts/u);
  for (const [path, contentType] of [["fonts.css", "text/css"], ["ready.js", "text/javascript"], ["MapleMonoBootstrap-Regular.woff2", "font/woff2"]]) {
    const response = await request(`/${path}`);
    assert.equal(response.statusCode, 200);
    assert.equal(response.headers["content-type"], contentType);
    assert.deepEqual(response.body, await readFile(resolve(fonts, path)));
  }
  assert.equal((await request("/missing.css")).statusCode, 404);
});
