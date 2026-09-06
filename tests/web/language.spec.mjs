import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("login language is consistent, switch is cancellable and persists without storing credentials", async ({ page }) => {
  await page.route("**/api/v2/**", route => route.fulfill({ status: 401, json: { code: "invalid_credentials", message: "SECRET", request_id: "lang-123" } }));
  await page.goto("/?lang=zh-CN");
  await expect(page.getByRole("heading", { name: "管理员登录" })).toBeVisible();
  await expect(page.getByLabel("用户名", { exact: true })).toBeVisible();
  await page.getByLabel("密码", { exact: true }).fill("unsaved-secret");
  const control = page.getByRole("button", { name: "切换为英文" });
  await control.click();
  const dialog = page.getByRole("dialog", { name: "切换语言" });
  await expect(dialog.getByRole("button", { name: "取消", exact: true })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(control).toBeFocused();
  await expect(page.getByLabel("密码", { exact: true })).toHaveValue("unsaved-secret");
  await control.click(); await dialog.getByRole("button", { name: "确认", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("heading", { name: "Administrator sign in" })).toBeVisible();
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue("");
  expect(await page.locator("body").innerText()).not.toMatch(/\p{Script=Han}/u);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("unsaved-secret");
  await page.goto("/"); await expect(page.locator("html")).toHaveAttribute("lang", "en");
  expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze()).violations).toEqual([]);
});
