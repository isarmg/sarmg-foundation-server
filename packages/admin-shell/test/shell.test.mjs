import test from"node:test";import assert from"node:assert/strict";import{readFile}from"node:fs/promises";test("shell owns sign in and request-safe failure copy",async()=>{const source=await readFile(new URL("../src/index.tsx",import.meta.url),"utf8");assert.match(source,/Administrator sign in/);assert.doesNotMatch(source,/localStorage|sessionStorage/);});
import { createElement as h } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { createSarmgAdminApplication, errorRequestId, LoginPage } from "../dist/index.js";

test("failure projection exposes only a validated Request ID", () => {
  assert.equal(errorRequestId({ message: "secret", requestId: "request-123" }), "request-123");
  assert.equal(errorRequestId({ requestId: "secret\npath" }), undefined);
  assert.equal(errorRequestId({ get requestId() { throw new Error("secret"); } }), undefined);
});
test("navigation rejects external targets, ambiguous paths and duplicate entries", () => {
  const base = { product: { name: "Test", version: "1.0" }, routes: null, client: {} };
  for (const href of ["https://example.com", "//example.com", "javascript:alert(1)", "/\\example.com", "/foo\nbar"]) {
    assert.throws(() => createSarmgAdminApplication({ ...base, navigation: [{ label: "Bad", href }] }), /Navigation/);
  }
  assert.throws(() => createSarmgAdminApplication({ ...base, navigation: [{ label: "A", href: "/a" }, { label: "B", href: "/a" }] }), /Navigation/);
  assert.equal(typeof createSarmgAdminApplication({ ...base, navigation: [{ label: "Good", href: "#overview" }] }), "function");
});
test("shared login renders labels and current credential bounds", () => {
  const html = renderToStaticMarkup(h(LoginPage, { async login() {} }));
  assert.match(html, /autocomplete="username"/i);
  assert.match(html, /maxlength="64"/i);
  assert.match(html, /autocomplete="current-password"/i);
  assert.match(html, /type="submit"/);
});
