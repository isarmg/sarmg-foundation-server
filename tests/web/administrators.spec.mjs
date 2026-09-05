import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("shared administrator panel safely creates, disables, changes password and ends its own session", async ({ page }) => {
  const id = "A".repeat(43);
  const session = { authenticated: true, user_id: id, username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  const records = [{ administrator_id: id, username: "admin", active: true, created_at_micros: 1, updated_at_micros: 2, last_login_at_micros: null }];
  let authenticated = true, failCreate = true, failDisable = true;
  await page.route("**/api/v2/**", async route => {
    const request = route.request(), url = new URL(request.url());
    const failure = (status, code) => route.fulfill({ status, contentType: "application/json", body: JSON.stringify({ code, message: "SECRET database internals", retryable: false, request_id: "management-123" }) });
    if (!authenticated) return failure(401, "auth.session_required");
    if (url.pathname.endsWith("/auth/session")) return route.fulfill({ json: session });
    expect(url.pathname.startsWith("/api/v2/platform/administrators")).toBe(true);
    if (request.method() === "GET") return route.fulfill({ json: records.slice(Number(url.searchParams.get("offset")), Number(url.searchParams.get("offset")) + Number(url.searchParams.get("limit"))) });
    expect(request.headers()["x-csrf-token"]).toBe(session.csrf_token);
    if (url.pathname.endsWith("/administrators")) {
      if (failCreate) { failCreate = false; return failure(409, "admin.conflict"); }
      expect(request.postDataJSON()).toEqual({ username: "secondary", password: "replacement password" });
      records.push({ ...records[0], administrator_id: "B".repeat(43), username: "secondary" });
    } else if (url.pathname.endsWith("/disable")) {
      expect(request.postData()).toBeNull();
      if (failDisable) { failDisable = false; return failure(409, "admin.last_administrator"); }
      records[1].active = false;
    } else if (url.pathname === `/api/v2/platform/administrators/${id}/password`) {
      expect(request.postDataJSON()).toEqual({ password: "replacement password" });
      authenticated = false;
    } else throw new Error(`Unexpected management route: ${url.pathname}`);
    return route.fulfill({ status: 204 });
  });
  await page.setViewportSize({ width: 360, height: 740 });
  await page.goto("/#administrators");
  await expect(page.getByRole("heading", { name: "Administrators", exact: true })).toBeVisible();
  for (const theme of ["light", "dark"]) {
    await page.getByLabel("Theme").selectOption(theme);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
  await page.getByRole("button", { name: "Create administrator", exact: true }).click();
  await page.getByLabel("Username", { exact: true }).fill("secondary");
  await page.getByLabel("New password", { exact: true }).fill("replacement password");
  await page.getByRole("button", { name: "Save administrator" }).click();
  await expect(page.getByRole("dialog")).toContainText("management-123");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await expect(page.getByLabel("New password")).toHaveValue("");
  await expect(page.getByLabel("New password")).toBeFocused();
  await page.getByLabel("New password").fill("replacement password");
  await page.getByRole("button", { name: "Save administrator" }).click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await page.getByRole("button", { name: "Disable secondary", exact: true }).click();
  await expect(page.getByRole("button", { name: "Cancel", exact: true })).toBeFocused();
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.getByRole("dialog").getByRole("alert")).toContainText("management-123");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await page.getByRole("button", { name: "Confirm", exact: true }).click();
  await expect(page.getByRole("button", { name: "Disable secondary", exact: true })).toBeDisabled();
  await page.getByRole("button", { name: "Change password for admin", exact: true }).click();
  await page.getByLabel("New password").fill("replacement password");
  await page.getByRole("button", { name: "Save administrator" }).click();
  await expect(page.getByRole("heading", { name: "Administrator sign in" })).toBeVisible();
  expect(await page.evaluate(() => ({ ...localStorage, ...sessionStorage }))).toEqual({});
});
