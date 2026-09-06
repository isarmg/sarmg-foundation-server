import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

const session = { authenticated: true, user_id: "018f1f4b-7a5d-7b5f-8d31-123456789abc", username: "admin", role: "admin", csrf_token: "A".repeat(43) };
async function mockApi(page, authenticated = false) {
  await page.route("**/api/v2/**", async route => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path.endsWith("/login")) {
      authenticated = request.postDataJSON().password === "correct-password";
    }
    if (path.endsWith("/logout")) {
      expect(request.headers()["x-csrf-token"]).toBe(session.csrf_token);
      authenticated = false; return route.fulfill({ status: 204 });
    }
    return route.fulfill({
      status: authenticated ? 200 : 401, contentType: "application/json",
      headers: { "x-request-id": "login-123" },
      body: JSON.stringify(authenticated ? session
        : { code: "invalid_credentials", message: "SECRET upstream response", request_id: "login-123" }),
    });
  });
}

test("failed login stays mounted, clears password, and displays only safe failure and Request ID", async ({ page }) => {
  await mockApi(page); await page.goto("/");
  await page.getByLabel("Username", { exact: true }).fill("admin");
  await page.getByLabel("Password", { exact: true }).fill("wrong-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("Sign in failed");
  await expect(page.getByRole("alert")).toContainText("login-123");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue("");
  await expect(page.getByLabel("Password", { exact: true })).toBeFocused();
  await page.getByLabel("Password", { exact: true }).fill("correct-password");
  await page.getByRole("button", { name: "Sign in", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Product overview" })).toBeVisible();
  await page.getByRole("button", { name: "Sign out", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Administrator sign in" })).toBeVisible();
});

test("modal traps keyboard focus, supports Escape and restores the trigger", async ({ page }) => {
  await mockApi(page, true); await page.goto("/");
  const trigger = page.getByRole("button", { name: "Open modal" });
  await trigger.click();
  const dialog = page.getByRole("dialog", { name: "Test modal" });
  await expect(dialog).toBeVisible();
  for (let index = 0; index < 8; index++) {
    await page.keyboard.press("Tab");
    expect(await dialog.evaluate(element => element.contains(document.activeElement))).toBe(true);
  }
  await page.keyboard.press("Escape");
  await expect(dialog).not.toBeVisible(); await expect(trigger).toBeFocused();
});

test("diagnostics is removed; bounded notifications, theme and error boundary remain", async ({ page }) => {
  const diagnosticRequests = [];
  page.on("request", request => { if (new URL(request.url()).pathname === "/api/v2/platform/diagnostics") diagnosticRequests.push(request.url()); });
  await mockApi(page, true); await page.goto("/");
  await expect(page.getByRole("button", { name: "Diagnostics", exact: true })).toHaveCount(0);
  for (let i = 0; i < 7; i++) await page.getByRole("button", { name: "Show notification" }).click();
  await expect(page.getByRole("button", { name: "Dismiss notification" })).toHaveCount(5);
  const notifications = await page.getByRole("region", { name: "Notifications", exact: true }).boundingBox();
  const product = await page.getByRole("heading", { name: "Product overview", exact: true }).boundingBox();
  expect(notifications.y + notifications.height).toBeLessThanOrEqual(product.y);
  await page.getByRole("button", { name: "Switch to dark mode" }).click();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "dark");
  await page.getByRole("button", { name: "Crash product route" }).click();
  await expect(page.getByRole("alert")).toContainText("render-123");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await expect(page.getByRole("button", { name: "Sign out", exact: true })).toBeVisible();
  expect(diagnosticRequests).toEqual([]);
});

test("shell has no WCAG AA violations or horizontal overflow at mobile width and dark theme", async ({ page }) => {
  await mockApi(page, true); await page.setViewportSize({ width: 360, height: 740 }); await page.goto("/");
  await expect(page.getByRole("heading", { name: "Product overview" })).toBeVisible();
  for (const theme of ["light", "dark"]) {
    if (await page.locator("html").getAttribute("data-theme") !== theme) await page.getByRole("button", { name: /Switch to .* mode/ }).click();
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});
