import { test, expect } from "@playwright/test";
test("administrator directory is not exposed; account settings remains available", async ({ page }) => {
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  await page.route("**/api/v2/auth/session", route => route.fulfill({ json: session }));
  await page.goto("/#administrators");
  await expect(page.getByRole("heading", { name: /Administrator account|管理员账号/ })).toHaveCount(0);
  await expect(page.getByRole("table", { name: /Administrator account|管理员账号/ })).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Account settings", exact: true })).toBeVisible();
});
