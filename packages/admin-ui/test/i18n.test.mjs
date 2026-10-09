import { test } from "node:test";
import assert from "node:assert/strict";
import { resolveLocale as getLocale, t, LANGUAGE_STORAGE_KEY } from "../dist/i18n.js";

test("locale precedence, storage failure, language fallback and safe interpolation", async () => {
  const previous = { window: globalThis.window, document: globalThis.document };
  try {
    let saved = null;
    globalThis.window = { location: { href: "https://example.test/admin/#users" }, navigator: { language: "zh-TW" }, localStorage: { getItem: key => { assert.equal(key, LANGUAGE_STORAGE_KEY); return saved; } } };
    globalThis.document = { documentElement: { lang: "" } };
    assert.equal(getLocale(), "zh-CN");
    const chinese = await import("../dist/i18n.js?chinese-test");
    chinese.initializeLanguage(); assert.equal(document.documentElement.lang, "zh-CN");
    saved = "en"; assert.equal(getLocale(), "en"); assert.equal(chinese.getLocale(), "zh-CN");
    window.location.href = "https://example.test/?lang=zh-CN#users";
    assert.equal(getLocale(), "zh-CN");
    assert.equal(chinese.t("名称：{0}", "Name: {0}", ['<img src=x onerror=alert(1)>{1}']), '名称：<img src=x onerror=alert(1)>{1}');
    window.location.href = "https://example.test/?lang=unsupported";
    window.localStorage.getItem = () => { throw new Error("blocked"); };
    window.navigator.language = "ja"; assert.equal(getLocale(), "en");
    assert.equal(t("{0} 台", "{0} hosts", [2]), "2 hosts");
  } finally {
    if (previous.window === undefined) delete globalThis.window; else globalThis.window = previous.window;
    if (previous.document === undefined) delete globalThis.document; else globalThis.document = previous.document;
  }
});

test("regional language tags normalize consistently and browser preference order is respected", async () => {
  const previous = { window: globalThis.window, document: globalThis.document };
  try {
    let saved = "zh-TW";
    globalThis.window = { location: { href: "https://example.test/?lang=EN-us#details" }, navigator: { language: "fr", languages: ["fr-FR", "zh-Hans", "en-US"] }, localStorage: { getItem: () => saved, setItem: (_, value) => { saved = value; } } };
    globalThis.document = { documentElement: { lang: "" } };
    assert.equal(getLocale(), "en");
    const english = await import("../dist/i18n.js?regional-test");
    assert.equal(document.documentElement.lang, "en");
    assert.equal(saved, "en");
    window.location.href = "https://example.test/?lang=zh-hant#details";
    assert.equal(getLocale(), "zh-CN");
    window.location.href = "https://example.test/?lang=unsupported";
    saved = "ZH-cn";
    assert.equal(getLocale(), "zh-CN");
    window.localStorage.getItem = () => { throw new Error("unavailable"); };
    assert.equal(getLocale(), "zh-CN");
    window.navigator.languages = ["fr-FR", "en-GB", "zh-CN"];
    assert.equal(getLocale(), "en");
    assert.equal(english.getLocale(), "en");
  } finally {
    if (previous.window === undefined) delete globalThis.window; else globalThis.window = previous.window;
    if (previous.document === undefined) delete globalThis.document; else globalThis.document = previous.document;
  }
});
