import { test, expect } from "@playwright/test";

async function openPreview(page, { ptz = false } = {}) {
  await page.goto(`/dialog-media.html?lang=en${ptz ? "&ptz=1" : ""}`);
  const trigger = page.getByRole("button", { name: "Open camera preview", exact: true });
  await trigger.focus();
  await page.keyboard.press("Enter");
  const dialog = page.getByRole("dialog", { name: "Fixed camera preview" });
  const video = dialog.locator("video");
  const close = dialog.getByRole("button", { name: "Close dialog", exact: true });
  await expect(dialog).toBeVisible();
  await expect(close).toBeFocused();
  await expect.poll(() => video.evaluate(element => element.readyState)).toBeGreaterThanOrEqual(2);
  // Default: exact non-PTZ shape. The query variant adds two ordinary PTZ buttons.
  await expect(dialog.locator("button")).toHaveCount(ptz ? 3 : 1);
  await expect(video).toHaveAttribute("controls", "");
  await page.evaluate(() => {
    window.mediaFocus = { outside: [], steps: [] };
    document.addEventListener("focusin", event => {
      if (document.querySelector("dialog[open]") && event.target instanceof Element && event.target.closest("[data-background]")) {
        window.mediaFocus.outside.push(event.target.getAttribute("data-background"));
      }
    });
  });
  return { trigger, dialog, video, close };
}
async function pressAndRecord(page, key) {
  await page.keyboard.press(key);
  return page.evaluate(key => {
    const active = document.activeElement;
    const state = { key, tag: active?.tagName, background: active instanceof Element && active.closest("[data-background]")?.getAttribute("data-background") || null, hasFocus: document.hasFocus(), inside: document.querySelector("dialog")?.contains(active) ?? false };
    window.mediaFocus.steps.push(state);
    return state;
  }, key);
}
async function assertNoBackgroundFocus(page) {
  expect(await page.evaluate(() => window.mediaFocus.outside)).toEqual([]);
  expect(await page.locator("[data-background]").evaluateAll(elements => elements.some(element => element === document.activeElement))).toBe(false);
}
async function attachFocus(page, testInfo) {
  await testInfo.attach("native-media-focus", { body: JSON.stringify(await page.evaluate(() => window.mediaFocus), null, 2), contentType: "application/json" });
}

test("video-only Dialog reaches and operates native media controls from the close button", async ({ page }, testInfo) => {
  const { dialog, video } = await openPreview(page);
  try {
    await pressAndRecord(page, "Tab");
    // The element is the DOM anchor for its native UI. Never use video.focus() or add tabindex.
    await expect.poll(() => video.evaluate(element => document.activeElement === element || element.matches(":focus-within"))).toBe(true);
    await assertNoBackgroundFocus(page);
    await page.keyboard.press("Space");
    await expect.poll(() => video.evaluate(element => element.paused)).toBe(false);
    await page.keyboard.press("Space");
    await expect.poll(() => video.evaluate(element => element.paused)).toBe(true);
    await expect(dialog).toBeVisible();
  } finally { await attachFocus(page, testInfo); }
});

test("media Dialog preserves background inertness across both native traversal boundaries", async ({ page }, testInfo) => {
  const { dialog, close } = await openPreview(page);
  try {
    // Browser-chrome stops are permitted. No background page control may receive focus.
    for (const key of ["Shift+Tab", "Tab", ...Array(12).fill("Tab"), ...Array(12).fill("Shift+Tab")]) {
      await pressAndRecord(page, key);
      await assertNoBackgroundFocus(page);
    }
    for (const element of await page.locator("[data-background]").all()) {
      await close.focus();
      await element.evaluate(node => node.focus());
      await assertNoBackgroundFocus(page);
    }
    const box = await page.getByRole("button", { name: "Background action", exact: true }).boundingBox();
    expect(box).not.toBeNull();
    await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    await expect(page.getByTestId("background-clicks")).toHaveText("0");
    await expect(dialog).toBeVisible();
    await assertNoBackgroundFocus(page);
  } finally { await attachFocus(page, testInfo); }
});

test("media Dialog Escape and close preserve trigger restoration over reopen", async ({ page }, testInfo) => {
  const { trigger, dialog, video, close } = await openPreview(page);
  try {
    await pressAndRecord(page, "Tab");
    await expect.poll(() => video.evaluate(element => document.activeElement === element || element.matches(":focus-within"))).toBe(true);
    await assertNoBackgroundFocus(page);
    await page.keyboard.press("Escape");
    await expect(dialog).not.toBeVisible();
    await expect(trigger).toBeFocused();
    await expect(page.getByTestId("closed-count")).toHaveText("1");
    await page.keyboard.press("Enter");
    await expect(dialog).toBeVisible();
    await expect(close).toBeFocused();
    await close.click();
    await expect(dialog).not.toBeVisible();
    await expect(trigger).toBeFocused();
    await expect(page.getByTestId("closed-count")).toHaveText("2");
  } finally { await attachFocus(page, testInfo); }
});
test("media Dialog native Tab reaches the video and following PTZ buttons", async ({ page }, testInfo) => {
  const { dialog, video } = await openPreview(page, { ptz: true });
  try {
    await pressAndRecord(page, "Tab");
    await expect.poll(() => video.evaluate(element => document.activeElement === element || element.matches(":focus-within"))).toBe(true);
    await assertNoBackgroundFocus(page);
    const visited = new Set();
    for (let step = 0; step < 20 && visited.size < 2; step++) {
      await pressAndRecord(page, "Tab");
      await assertNoBackgroundFocus(page);
      const focused = await page.evaluate(() => document.activeElement?.getAttribute("data-testid"));
      if (focused === "ptz-pan-left" || focused === "ptz-stop") visited.add(focused);
    }
    expect([...visited]).toEqual(["ptz-pan-left", "ptz-stop"]);
    await expect(dialog).toBeVisible();
  } finally { await attachFocus(page, testInfo); }
});
