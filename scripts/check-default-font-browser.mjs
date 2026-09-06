import assert from "node:assert/strict";
import { createServer } from "node:http";
import { readFile, readdir } from "node:fs/promises";
import { resolve, extname, join } from "node:path";
import { chromium, firefox } from "@playwright/test";

for (const argument of process.argv.slice(2)) {
  const provenance = JSON.parse(await readFile(resolve(argument, "clients/web/fonts/provenance.json"), "utf8"));
  const latinName = provenance.latin?.variant === "Normal NL" ? "MapleMonoNormalNL-Regular" : "MapleMono-Italic";
  const root = resolve(argument, "clients/web/dist");
  const styles = [];
  for (const entry of await readdir(root, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(".css")) styles.push(entry.name);
    if (entry.name === "assets" && entry.isDirectory()) {
      for (const name of await readdir(join(root, "assets"))) if (name.endsWith(".css")) styles.push(`assets/${name}`);
    }
  }
  assert.ok(styles.length > 0);
  const server = createServer(async (request, response) => {
    try {
      const pathname = new URL(request.url, "http://localhost").pathname.replace(/^\/admin\//, "/");
      if (pathname === "/font-check") {
        response.setHeader("content-type", "text/html; charset=utf-8");
        response.end(`<!doctype html><html data-sarmg-appearance="content-blocks"><head>${styles.map(file => `<link rel="stylesheet" href="/${file}">`).join("")}</head><body data-sarmg-scope><p id="latin">Sunshine fi fl al ul != ===</p><p id="chinese">中文管理字体測試</p><p id="japanese" lang="ja">日本語かなカナ</p><p id="bold" style="font-weight:700">中文日本語</p><input value="Sunshine 中文 日本語"></body></html>`);
        return;
      }
      const file = resolve(root, `.${decodeURIComponent(pathname)}`);
      if (!file.startsWith(root + "/")) throw new Error("outside fixture");
      response.setHeader("content-type", ({ ".css": "text/css", ".woff2": "font/woff2", ".js": "application/javascript" })[extname(file)] ?? "application/octet-stream");
      response.end(await readFile(file));
    } catch { response.statusCode = 404; response.end(); }
  });
  await new Promise(done => server.listen(0, "127.0.0.1", done));
  try {
    for (const [name, engine] of [["chromium", chromium], ["firefox", firefox]]) {
      const browser = await engine.launch();
      try {
        const page = await browser.newPage();
        const failures = [];
        page.on("requestfailed", request => failures.push(request.url()));
        page.on("response", response => { if (response.status() >= 400) failures.push(response.url()); });
        await page.goto(`http://127.0.0.1:${server.address().port}/font-check`);
        await page.evaluate(async () => {
          await document.fonts.load('400 18px "Sarmg Maple"', "Sunshine 中文管理字体測試日本語かなカナ");
          await document.fonts.load('700 18px "Sarmg Maple"', "中文日本語");
          await document.fonts.ready;
          await new Promise(done => requestAnimationFrame(() => requestAnimationFrame(done)));
        });
        for (const selector of ["#latin", "#chinese", "#japanese", "#bold", "input"]) {
          const style = await page.locator(selector).evaluate(element => {
            const css = getComputedStyle(element);
            return { family: css.fontFamily, ligatures: css.fontVariantLigatures, features: css.fontFeatureSettings, style: css.fontStyle };
          });
          assert.ok(style.family.includes("Sarmg Maple"), `${argument} ${selector}`);
          assert.equal(style.ligatures, "none");
          assert.ok(style.features.includes('"calt" 0'));
          assert.equal(style.style, "normal");
        }
        if (name === "chromium") {
          const cdp = await page.context().newCDPSession(page);
          await cdp.send("DOM.enable");
          await cdp.send("CSS.enable");
          const { root: document } = await cdp.send("DOM.getDocument");
          for (const [selector, expected] of [["#latin", latinName], ["#chinese", "MapleMonoNL-CN-Regular"], ["#japanese", "MapleMonoNL-CN-Regular"], ["#bold", "MapleMonoNL-CN-Bold"]]) {
            const { nodeId } = await cdp.send("DOM.querySelector", { nodeId: document.nodeId, selector });
            const { fonts } = await cdp.send("CSS.getPlatformFontsForNode", { nodeId });
            assert.ok(fonts.length > 0);
            assert.ok(fonts.every(font => font.isCustomFont && font.postScriptName === expected), JSON.stringify({ selector, fonts }));
            if (selector === "#latin") {
              assert.equal(fonts.reduce((total, font) => total + font.glyphCount, 0), (await page.locator(selector).textContent()).length);
            }
          }
        }
        assert.deepEqual(failures, []);
        console.log(`${argument}: ${name} ${latinName} + Chinese + Japanese + bold + form controls; no ligatures or missing font requests`);
      } finally { await browser.close(); }
    }
  } finally { await new Promise(done => server.close(done)); }
}
