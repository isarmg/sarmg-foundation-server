import test from"node:test";import assert from"node:assert/strict";import{readFile}from"node:fs/promises";test("accessibility policies ship",async()=>{const css=await readFile(new URL("../styles.css",import.meta.url),"utf8");for(const rule of ["focus-visible","forced-colors","prefers-reduced-motion"])assert.match(css,new RegExp(rule));});
import { createElement as h } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { Button, Checkbox, Dialog, ErrorState, IconButton, LoadingState, RequestId, Table } from "../dist/index.js";

test("buttons are non-submitting by default and icon labels are required", () => {
  assert.match(renderToStaticMarkup(h(Button, null, "Action")), /type="button"/);
  assert.match(renderToStaticMarkup(h(Button, { type: "submit" }, "Save")), /type="submit"/);
  assert.throws(() => renderToStaticMarkup(h(IconButton, { "aria-label": " " }, "X")), /aria-label/);
  assert.match(renderToStaticMarkup(h(Checkbox, { type: "text" })), /type="checkbox"/);
});
test("loading and errors have accessible visible content and bounded Request IDs", () => {
  assert.match(renderToStaticMarkup(h(LoadingState)), /role="status".*Loading/);
  assert.match(renderToStaticMarkup(h(ErrorState, { requestId: "request-123" }, "Failed")), /role="alert".*Request ID/);
  assert.equal(renderToStaticMarkup(h(RequestId, { value: "path\nsecret" })), "");
  assert.equal(renderToStaticMarkup(h(RequestId, { value: "x".repeat(129) })), "");
});
test("dialog uses native modal element and a programmatically associated title", () => {
  const markup = renderToStaticMarkup(h(Dialog, { title: "Confirm action", onClose() {} }, "Details"));
  assert.match(markup, /^<dialog/);
  const id = /aria-labelledby="([^"]+)"/.exec(markup)[1];
  assert.ok(markup.includes(`id="${id}"`));
  assert.match(renderToStaticMarkup(h(Table, { "aria-label": "Hosts" })), /tabindex="0".*aria-label="Hosts"/);
});
