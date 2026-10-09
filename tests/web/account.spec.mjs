import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";

test("account menu verifies current password, clears secrets and returns to sign in", async ({ page }) => {
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  let authenticated = true;
  let updates = 0;
  await page.route("**/api/v1/**", async route => {
    const request = route.request(), path = new URL(request.url()).pathname;
    if (path.endsWith("/auth/session")) return route.fulfill({ status: authenticated ? 200 : 401, json: authenticated ? session : { code: "auth.session_required", message: "Sign in required", retryable: false } });
    expect(path).toBe("/api/v1/platform/administrators/self");
    expect(request.headers()["x-csrf-token"]).toBe(session.csrf_token);
    updates++;
    const input = request.postDataJSON();
    expect(input.username).toBe("renamed");
    if (input.current_password !== "correct horse battery") return route.fulfill({ status: 403, json: { code: "admin.current_password_invalid", message: "SECRET server details", retryable: false } });
    expect(input.new_password).toBe("updated correct password");
    authenticated = false;
    return route.fulfill({ status: 204 });
  });
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  const trigger = page.getByRole("button", { name: "Account settings", exact: true });
  await trigger.click();
  const dialog = page.getByRole("region", { name: "Account settings", exact: true });
  await expect(page.getByRole("dialog")).toHaveCount(0);
  await expect(dialog).toBeVisible();
  expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  await page.getByLabel("Username", { exact: true }).fill("renamed");
  await page.getByLabel("Current password", { exact: true }).fill("incorrect password");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(dialog).toContainText("current password is incorrect");
  await expect(page.getByLabel("Current password", { exact: true })).toHaveValue("");
  await expect(page.getByLabel("New password", { exact: true })).toHaveValue("");
  await expect(page.getByLabel("Confirm new password", { exact: true })).toHaveValue("");
  await expect(page.locator("body")).not.toContainText("SECRET");
  await page.getByLabel("Current password", { exact: true }).fill("correct horse battery");
  await page.getByLabel("New password", { exact: true }).fill("updated correct password");
  await page.getByLabel("Confirm new password", { exact: true }).fill("different password");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(dialog).toContainText("new passwords do not match");
  expect(updates).toBe(1);
  await page.getByLabel("Confirm new password", { exact: true }).fill("updated correct password");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByRole("heading", { name: "Administrator sign in" })).toBeVisible();
  await expect(page.getByRole("status")).toContainText("Account updated");
  expect(updates).toBe(2);
  expect(await page.evaluate(() => ({ ...localStorage, ...sessionStorage }))).toEqual({});
});

test("ordinary account route keeps the person icon unchanged, clears drafts and shares persistent login themes", async ({ page }) => {
  test.setTimeout(120_000);
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  let authenticated = false, updates = 0;
  await page.route("**/api/v1/**", route => {
    const path = new URL(route.request().url()).pathname;
    if (path.endsWith("/auth/session")) return route.fulfill({ status: authenticated ? 200 : 401, json: authenticated ? session : {code:"auth.session_required"} });
    updates++;
    return route.fulfill({status:403,headers:{"x-request-id":"private-id"},json:{code:"admin.current_password_invalid",message:"SECRET",request_id:"private-id",retryable:false}});
  });
  const appearance = element => {
    const style = getComputedStyle(element), svg = element.querySelector("svg"), box = svg.getBoundingClientRect();
    return { color:style.color, background:style.backgroundColor, decoration:style.textDecorationLine, width:box.width, height:box.height, image:svg.innerHTML };
  };
  for (const language of ["en", "zh-CN"]) {
    const english = language === "en";
    authenticated = false;
    await page.goto(`/?lang=${language}`);
    await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "ready", { timeout: 30_000 });
    const login = page.locator(".xcss-auth-card"), controls = page.getByRole("group",{name:english?"Display settings":"显示设置"});
    await expect(login).toBeVisible();
    for (const theme of ["light","dark"]) {
      if (await page.locator("html").getAttribute("data-theme") !== theme) await controls.getByRole("button", {name:english?/Switch to .* mode/:/切换到.*模式/}).click();
      const colors = await login.evaluate(node=>({background:getComputedStyle(node).backgroundColor,text:[...node.querySelectorAll(".xcss-form-field > span,input,button")].map(n=>getComputedStyle(n).color)}));
      expect(colors.background).toBe(theme==="light"?"rgb(242, 242, 242)":"rgb(37, 44, 49)");
      if(theme==="light") expect(colors.text.every(color=>color==="rgb(0, 0, 0)")).toBe(true);
      await expect(controls.getByRole("button")).toHaveCount(2);
      for(const svg of await controls.locator("svg").all()) expect(await svg.evaluate(n=>n.getBoundingClientRect().width)).toBe(16);
    }
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "ready", { timeout: 30_000 });
    await expect(page.locator("html")).toHaveAttribute("data-theme","dark");
    authenticated = true;
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "ready", { timeout: 30_000 });
    const trigger = page.getByRole("banner").getByRole("button", {name:english?"Account settings":"账号设置",exact:true});
    await expect(trigger).toBeVisible();
    await page.mouse.move(0,0);
    const original = await trigger.evaluate(appearance);
    let fonts = 0; page.on("request", request=>{if(new URL(request.url()).pathname.endsWith(".woff2"))fonts++;});
    await trigger.click(); await page.mouse.move(0,0);
    const account = page.getByRole("region", {name:english?"Account settings":"账号设置",exact:true});
    await expect(account).toBeVisible();
    await expect(page).toHaveURL(/#account$/);
    await expect(page.getByRole("dialog")).toHaveCount(0);
    expect(await trigger.evaluate(appearance)).toEqual(original);
    expect(original.decoration).toBe("none");
    expect(await trigger.getAttribute("aria-pressed")).toBeNull();
    expect(await trigger.getAttribute("aria-current")).toBeNull();
    const labels = english?["Username","Current password","New password","Confirm new password"]:["用户名","当前密码","新密码","确认新密码"];
    for(const width of [1280,360,320]) {
      await page.setViewportSize({width,height:800});
      await expect(account.locator(".xcss-form-field > span")).toHaveText(labels);
      for(const theme of ["light","dark"]) {
        if(await page.locator("html").getAttribute("data-theme")!==theme) await page.getByRole("banner").getByRole("button",{name:english?/Switch to .* mode/:/切换到.*模式/}).click();
        expect(await account.locator(".xcss-content-panel").evaluate(n=>getComputedStyle(n).backgroundColor)).toBe(theme==="light"?"rgb(242, 242, 242)":"rgb(37, 44, 49)");
        expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
        expect((await new AxeBuilder({page}).withTags(["wcag2a","wcag2aa","wcag21aa"]).analyze()).violations).toEqual([]);
      }
    }
    await account.getByLabel(labels[1],{exact:true}).fill("incorrect-password");
    await account.getByRole("button",{name:english?"Save":"保存",exact:true}).click();
    await expect(account.getByRole("alert")).toHaveText(english?"The current password is incorrect.":"当前密码不正确。");
    for (const width of [320, 360, 1280]) {
      await page.setViewportSize({ width, height: 800 });
      expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
      for (const input of await account.locator("input").all()) {
        expect(await input.evaluate(node=>node.getBoundingClientRect().width)).toBeGreaterThan(48);
      }
    }
    await expect(account).not.toContainText("private-id");
    await expect(account.locator(".xcss-request-id")).toHaveCount(0);
    await account.getByLabel(labels[1],{exact:true}).fill("unsubmitted-password");
    await page.getByRole("link",{name:"Overview",exact:true}).click();
    await expect(account).toHaveCount(0);
    await trigger.click();
    await expect(account.getByLabel(labels[1],{exact:true})).toHaveValue("");
    expect(fonts).toBe(0);
  }
  expect(updates).toBe(2);
});
