import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { extname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { chromium, firefox, expect } from "@playwright/test";

process.env.PW_TEST_SCREENSHOT_NO_FONTS_READY = "1";
const repository = resolve(import.meta.dirname, "..");
const canonicalFonts = join(repository, "web/web-fonts");
const expectedFaces = (await readFile(join(canonicalFonts, "fonts.css"), "utf8")).match(/@font-face\{/gu).length;
const boot = await readFile(join(canonicalFonts, "boot.css"), "utf8");
const text = "已加载中文管理字体 鹤龘鬱 日本語 ABC 0123456789";

async function assertPlainBackground(page, dark = false) {
  const screenshot = await page.screenshot();
  const pixels = await page.evaluate(async encoded => {
    const bitmap = await createImageBitmap(new Blob([Uint8Array.from(atob(encoded), c => c.charCodeAt(0))], { type: "image/png" }));
    const canvas = document.createElement("canvas"); canvas.width = bitmap.width; canvas.height = bitmap.height;
    const context = canvas.getContext("2d"); context.drawImage(bitmap, 0, 0);
    const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
    const color = [...data.subarray(0, 4)];
    let uniform = true;
    for (let index = 0; index < data.length; index++) if (data[index] !== color[index % 4]) { uniform = false; break; }
    return { color, uniform };
  }, screenshot.toString("base64"));
  assert.deepEqual(pixels, { color: dark ? [21, 26, 29, 255] : [255, 255, 255, 255], uniform: true });
  assert.equal(await page.locator("body").evaluate(body => getComputedStyle(body).visibility), "hidden");
}

export function parseFontStartupOptions(args) {
  const { values } = parseArgs({ args, options: {
    source: { type: "string" },
    url: { type: "string", multiple: true },
    "color-scheme": { type: "string", default: "light" },
    "storage-state": { type: "string" },
  } });
  const urls = values.url ?? [];
  if (Boolean(values.source) === (urls.length > 0)) {
    throw new Error("Pass either --source FONT_DIRECTORY or one or more --url PAGE_URL options");
  }
  const colorScheme = values["color-scheme"];
  if (colorScheme !== "light" && colorScheme !== "dark") {
    throw new Error("--color-scheme must be light or dark");
  }
  for (const value of urls) {
    const url = new URL(value);
    if (url.protocol !== "http:" && url.protocol !== "https:") {
      throw new Error("--url must use HTTP or HTTPS");
    }
  }
  return { source: values.source && resolve(values.source), urls, colorScheme, storageState: values["storage-state"] };
}

// Only the common font fixture is hosted here. Consumers own their server,
// page templates, API fixtures, authentication and URL selection.
export function fontSourceHandler(directory) {
  const files = resolve(directory);
  return async (request, response) => {
    try {
      const pathname = decodeURIComponent(new URL(request.url, "http://localhost").pathname);
      if (pathname === "/font-startup") {
        response.setHeader("content-type", "text/html; charset=utf-8");
        response.end(`<!doctype html><html data-xcss-fonts="pending"><head><style>${boot}</style><link rel="stylesheet" href="/fonts.css"></head><body><p id="source-content" hidden>${text}</p><script type="module">import {startAfterFonts} from '/ready.js';void startAfterFonts(()=>{document.querySelector('#source-content').hidden=false;});</script></body></html>`); return;
      }
      const file = resolve(files, `.${pathname}`);
      if (!file.startsWith(files + "/")) throw new Error("outside font fixture");
      response.setHeader("content-type", ({ ".css": "text/css", ".js": "text/javascript", ".html": "text/html", ".woff2": "font/woff2" })[extname(file)] ?? "application/octet-stream");
      response.end(await readFile(file));
    } catch { response.statusCode = 404; response.end(); }
  };
}

export async function checkFontStartup(args = process.argv.slice(2)) {
  const { source, urls, colorScheme, storageState } = parseFontStartupOptions(args);
  let server;
  if (source) {
    // Fail early on an invalid source directory, before launching a browser.
    await readFile(join(source, "fonts.css"));
    server = createServer(fontSourceHandler(source));
    await new Promise((done, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", done);
    });
    urls.push(`http://127.0.0.1:${server.address().port}/font-startup`);
  }
  try {
    for (const engine of [chromium, firefox]) {
      const browser = await engine.launch();
      try {
        for (const url of urls) {
          const context = await browser.newContext({ colorScheme, storageState, viewport: { width: 360, height: 740 } });
          const page = await context.newPage();
          let releaseBootstrap, releaseShards;
          const bootstrapGate = new Promise(done => { releaseBootstrap = done; });
          const shardGate = new Promise(done => { releaseShards = done; });
          const requests = new Map(), errors = [];
          let heldBootstrap = 0, heldShards = 0;
          page.on("pageerror", error => errors.push(error.message));
          page.on("request", request => { if (request.url().endsWith(".woff2")) requests.set(request.url(), (requests.get(request.url()) ?? 0) + 1); });
          await page.route(/MapleMonoBootstrap-Regular[^/]*\.woff2$/u, async route => { heldBootstrap++; await bootstrapGate; await route.continue(); });
          await page.route(/MapleMonoNL-CN-(Regular|Bold)-9CCF-9E67[^/]*\.woff2$/u, async route => { heldShards++; await shardGate; await route.continue(); });
          try {
            await page.goto(url, { waitUntil: "domcontentloaded" });
            await expect.poll(() => ({ bootstrap: heldBootstrap, shards: heldShards }), { timeout: 15_000 }).toEqual({ bootstrap: 1, shards: 2 });
            await page.waitForTimeout(1500);
            await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "pending");
            await assertPlainBackground(page, colorScheme === "dark");
            releaseBootstrap();
            await expect.poll(() => page.evaluate(() => [...document.fonts].filter(face => face.family.includes("Bootstrap") && face.weight === "400").every(face => face.status === "loaded"))).toBe(true);
            await assertPlainBackground(page, colorScheme === "dark");
            releaseShards();
            await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "ready", { timeout: 15_000 });
            await expect.poll(() => page.locator("body").innerText()).not.toBe("");
            const decoded = await page.evaluate(() => ({ loaded: [...document.fonts].every(face => face.status === "loaded"), unique: new Set([...document.fonts].map(face => `${face.family}:${face.weight}:${face.unicodeRange}`)).size }));
            assert.deepEqual(decoded, { loaded: true, unique: expectedFaces });
            assert.equal(requests.size, expectedFaces);
            assert.ok([...requests.values()].every(count => count === 1), "each font file must load exactly once");
            const before = requests.size;
            await page.evaluate(value => {
              const probe = document.createElement("section"); probe.id = "font-menu-probe";
              for (const weight of [400, 700]) {
                const text = document.createElement("p"); text.textContent = value;
                text.style.fontFamily = '"Sarmg Maple"'; text.style.fontWeight = String(weight); probe.append(text);
              }
              document.body.append(probe);
            }, text);
            const probe = page.locator("#font-menu-probe");
            await expect(probe).toBeVisible();
            const painted = await probe.screenshot();
            for (let toggle = 0; toggle < 3; toggle++) {
              await probe.evaluate(node => { node.hidden = true; });
              await probe.evaluate(node => { node.hidden = false; });
            }
            assert.ok((await probe.screenshot()).equals(painted), "later text cannot flash or change font");
            assert.equal(requests.size, before, "later menu text cannot download fonts");
            assert.ok([...requests.values()].every(count => count === 1));
            assert.deepEqual(errors, []);
            console.log(`${url}: ${engine.name()} plain background while pending; all ${expectedFaces} fonts decoded before UI; later text stable`);
          } finally { releaseBootstrap(); releaseShards(); await context.close(); }
        }
        // A broken font must never release the UI or activate a timeout fallback.
        const page = await browser.newPage({ colorScheme, storageState, viewport: { width: 360, height: 740 } });
        const failures = [];
        page.on("console", message => { if (message.text().includes("Application font preparation failed")) failures.push(message.text()); });
        await page.route(/MapleMonoNL-CN-Regular-9CCF-9E67[^/]*\.woff2$/u, route => route.fulfill({ status: 503, body: "" }));
        await page.goto(urls.at(-1), { waitUntil: "domcontentloaded" });
        await expect.poll(() => failures.length, { timeout: 15_000 }).toBe(1);
        await page.waitForTimeout(1500);
        await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "pending");
        await assertPlainBackground(page, colorScheme === "dark");
        console.log(`${urls.at(-1)}: ${engine.name()} failed font keeps the background without system-font fallback`);
      } finally { await browser.close(); }
    }
  } finally { if (server) await new Promise(done => server.close(done)); }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await checkFontStartup();
}
