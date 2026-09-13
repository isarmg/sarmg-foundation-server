import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

for (const [locale, usernameLabel, passwordLabel, submitLabel, usernameError, passwordError] of [
  ["en", "Username", "Password", "Sign in", "Enter your username.", "Enter your password."],
  ["zh-CN", "用户名", "密码", "登录", "请输入用户名。", "请输入密码。"],
]) {
  test(`login validation uses the inline error row without native popups (${locale})`, async ({ page }) => {
    let loginRequests = 0;
    await page.route("**/api/v2/**", route => {
      if (route.request().method() === "POST") loginRequests++;
      return route.fulfill({ status: 401, json: { code: "invalid_credentials", request_id: "inline-login" } });
    });
    await page.goto(`/?lang=${locale}`);
    const username = page.getByLabel(usernameLabel, { exact: true });
    const password = page.getByLabel(passwordLabel, { exact: true });
    const submit = page.getByRole("button", { name: submitLabel, exact: true });
    const error = page.getByRole("alert");
    await expect(username).toBeVisible();
    await page.evaluate(() => {
      window.nativeInvalidCount = 0;
      document.addEventListener("invalid", () => window.nativeInvalidCount++, true);
    });
    await submit.click();
    await expect(error).toHaveText(usernameError);
    await expect(username).toBeFocused();
    await expect(username).toHaveAttribute("aria-invalid", "true");
    await expect(username).toHaveAccessibleDescription(usernameError);
    await username.fill("admin");
    await expect(error).toHaveCount(0);
    await password.press("Enter");
    await expect(error).toHaveText(passwordError);
    await expect(password).toBeFocused();
    await expect(password).toHaveAccessibleDescription(passwordError);
    expect(await page.locator("form").evaluate(form => form.noValidate)).toBe(true);
    expect(await page.evaluate(() => window.nativeInvalidCount)).toBe(0);
    expect(loginRequests).toBe(0);
    // The same order is meaningful without the optional content-blocks theme.
    expect(await error.evaluate(node => Boolean(node.compareDocumentPosition(document.querySelector('button[type="submit"]')) & Node.DOCUMENT_POSITION_FOLLOWING))).toBe(true);
    await password.fill("valid-length-password");
    await expect(error).toHaveCount(0);
    await submit.click();
    await expect(error).toContainText(locale === "en" ? "Sign in failed." : "登录失败");
    expect(loginRequests).toBe(1);
    await expect(password).toHaveValue("");
  });
}

test("login language is consistent, switches immediately and persists without storing credentials", async ({ page }) => {
  await page.route("**/api/v2/**", route => route.fulfill({ status: 401, json: { code: "invalid_credentials", message: "SECRET", request_id: "lang-123" } }));
  await page.goto("/?lang=zh-CN");
  await expect(page.getByRole("heading", { name: "管理员登录" })).toBeVisible();
  await expect(page.getByLabel("用户名", { exact: true })).toBeVisible();
  await page.getByLabel("密码", { exact: true }).fill("unsaved-secret");
  await page.getByLabel("用户名", { exact: true }).fill("admin");
  const control = page.getByRole("button", { name: "切换为英文" });
  await page.evaluate(() => { history.replaceState(null, "", "/"); localStorage.setItem("sarmg.admin.language", "en"); });
  await page.getByRole("button", { name: "登录", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("登录失败");
  await page.getByLabel("密码", { exact: true }).fill("unsaved-secret");
  await control.click();
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("heading", { name: "Administrator sign in" })).toBeVisible();
  await expect(page.getByLabel("Password", { exact: true })).toHaveValue("");
  expect(await page.locator("body").innerText()).not.toMatch(/\p{Script=Han}/u);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain("unsaved-secret");
  await page.goto("/"); await expect(page.locator("html")).toHaveAttribute("lang", "en");
  expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze()).violations).toEqual([]);
});
