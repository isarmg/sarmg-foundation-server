import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
async function api(page) {
  let authenticated = false;
  await page.route("**/api/v2/**", async route => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path.endsWith("/login")) authenticated = request.postDataJSON().password === "correct-password";
    if (path.endsWith("/logout")) {
      expect(request.headers()["x-csrf-token"]).toBe(session.csrf_token);
      authenticated = false;
      return route.fulfill({ status: 204 });
    }
    return route.fulfill({ status: authenticated ? 200 : 401, contentType: "application/json",
      headers: { "x-request-id": "appearance-123" },
      body: JSON.stringify(authenticated ? session : { code: "invalid_credentials", message: "SECRET", request_id: "appearance-123" }) });
  });
}

test("base stylesheet defaults to six-row cards; consumers can opt out and design their own appearance", async ({ page }) => {
  await api(page); await page.goto("/");
  const card = page.locator(".sarmg-auth-card");
  await expect(page.getByLabel("Username", { exact: true })).toBeVisible();
  expect(await card.evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("0px");
  expect((await card.boundingBox()).width).toBe(380);
  for (const width of [1280, 360, 320]) {
    await page.setViewportSize({ width, height: 740 });
    for (const theme of ["light", "dark"]) {
      await page.evaluate(theme => { document.documentElement.dataset.sarmgAppearance = "content-blocks"; document.documentElement.dataset.theme = theme; }, theme);
      const rect = await card.boundingBox();
      expect(rect.width).toBe(Math.min(380, width - 48));
      expect(rect.width / rect.height).toBeCloseTo(1.5, 2);
      expect(await card.evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("0px");
      const input = await page.getByLabel("Username", { exact: true }).boundingBox();
      const password = await page.getByLabel("Password", { exact: true }).boundingBox();
      const submit = await page.getByRole("button", { name: "Sign in", exact: true }).boundingBox();
      expect(input.y - rect.y).toBeCloseTo(rect.height / 6, 0);
      expect(password.y - rect.y).toBeCloseTo(rect.height / 2, 0);
      expect(submit.y - rect.y).toBeCloseTo(rect.height * 5 / 6, 0);
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
      expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    }
  }
  await page.evaluate(() => { document.documentElement.dataset.sarmgAppearance = "custom"; });
  expect(await card.evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("1px");
  await page.addStyleTag({ content: 'html[data-sarmg-appearance="custom"] .sarmg-auth-card { border-radius: 0; background: rgb(240, 240, 240); }' });
  expect(await card.evaluate(el => getComputedStyle(el).borderRadius)).toBe("0px");
  await page.evaluate(() => delete document.documentElement.dataset.sarmgAppearance);
  expect(await card.evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("0px");
});

test("default appearance preserves failure handling, login/logout, theme and modal keyboard behavior", async ({ page }) => {
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await api(page); await page.goto("/");
  await page.getByLabel("Username", { exact: true }).fill("admin");
  await page.getByLabel("Password", { exact: true }).fill("wrong-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("appearance-123");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue("");
  await expect(page.getByLabel("Password", { exact: true })).toBeFocused();
  await page.getByLabel("Password", { exact: true }).fill("correct-password");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Product overview" })).toBeVisible();
  await page.setViewportSize({ width: 360, height: 740 });
  for (const theme of ["light", "dark"]) {
    if (await page.locator("html").getAttribute("data-theme") !== theme) await page.getByRole("button", { name: /Switch to .* mode/ }).click();
    const trigger = page.getByRole("button", { name: "Open modal" });
    await trigger.click();
    const dialog = page.getByRole("dialog", { name: "Test modal" });
    for (let i = 0; i < 6; i++) {
      await page.keyboard.press("Tab");
      expect(await dialog.evaluate(el => el.contains(document.activeElement))).toBe(true);
    }
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    await page.keyboard.press("Escape");
    await expect(trigger).toBeFocused();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByLabel("Password", { exact: true })).toBeVisible();
  await page.emulateMedia({ forcedColors: "active" });
  expect(await page.locator(".sarmg-auth-card").evaluate(el => getComputedStyle(el).borderTopWidth)).toBe("1px");
  expect(errors).toEqual([]);
});

test("menu-to-content and content-to-subheading spacing use one Foundation default", async ({ page }) => {
  await api(page); await page.goto("/");
  await page.getByLabel("Username", { exact: true }).fill("admin");
  await page.getByLabel("Password", { exact: true }).fill("correct-password");
  await page.keyboard.press("Enter");
  await expect(page.getByRole("heading", { name: "Product overview" })).toBeVisible();
  await page.locator(".sarmg-shell-main > section").evaluate(section => {
    const anchor = document.createElement("div");
    anchor.className = "sarmg-table-scroll";
    anchor.dataset.spacingAnchor = "true";
    anchor.textContent = "Previous section content";
    const heading = document.createElement("h2");
    heading.dataset.spacingSubheading = "true";
    heading.textContent = "Following subsection";
    section.append(anchor, heading);
  });
  for (const width of [1280, 360]) {
    await page.setViewportSize({ width, height: 740 });
    const spacing = await page.evaluate(() => {
      const header = document.querySelector(".sarmg-page-header");
      const first = document.querySelector(".sarmg-shell-main > section > h1");
      const anchor = document.querySelector("[data-spacing-anchor]");
      const subheading = document.querySelector("[data-spacing-subheading]");
      if (!header || !first || !anchor || !subheading) throw new Error("Spacing fixture is incomplete");
      return {
        token: getComputedStyle(document.documentElement).getPropertyValue("--sarmg-content-spacing").trim(),
        menuToFirst: first.getBoundingClientRect().top - header.getBoundingClientRect().bottom,
        contentToSubheading: subheading.getBoundingClientRect().top - anchor.getBoundingClientRect().bottom,
      };
    });
    expect(spacing.token).toBe("16px");
    expect(spacing.menuToFirst).toBeCloseTo(16, 1);
    expect(spacing.contentToSubheading).toBeCloseTo(16, 1);
  }
});
