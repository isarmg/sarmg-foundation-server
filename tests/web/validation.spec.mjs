import { test, expect } from "@playwright/test";

test("loading an existing record clears obsolete required-field errors", async ({ page }) => {
  await page.route("**/api/v1/**", route => route.fulfill({ json: { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) } }));
  await page.goto("/?lang=en#validation");
  const name = page.getByRole("textbox", { name: "Name", exact: true });
  const category = page.getByRole("combobox", { name: "Category", exact: true });
  await page.getByRole("button", { name: "Save", exact: true }).click();
  for (const field of [name, category]) {
    await expect.poll(() => field.evaluate(element => element.validity.customError)).toBe(true);
  }
  await expect(page.getByTestId("saved-count")).toHaveText("0");
  await page.getByRole("button", { name: "Edit existing", exact: true }).click();
  for (const field of [name, category]) {
    await expect.poll(() => field.evaluate(element => element.validity.valid)).toBe(true);
  }
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByTestId("saved-count")).toHaveText("1");
  await page.getByRole("button", { name: "New", exact: true }).click();
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByTestId("saved-count")).toHaveText("1");
  await name.fill("existing");
  await page.getByRole("button", { name: "Save", exact: true }).click();
  await expect(page.getByTestId("saved-count")).toHaveText("2");
});

test("controlled instance names retain caller validation after an invalid check", async ({ page }) => {
  await page.route("**/api/v1/**", route => route.fulfill({ json: { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) } }));
  await page.goto("/?lang=en#validation");
  const name = page.getByRole("textbox", { name: "Instance name", exact: true });
  const save = page.getByRole("button", { name: "Save instance", exact: true });
  // Dispatch invalid without a native validation popup intercepting the next click.
  expect(await name.evaluate(element => element.checkValidity())).toBe(false);
  await expect.poll(() => name.evaluate(element => element.validity.customError)).toBe(true);
  await page.getByRole("button", { name: "Use whitespace name", exact: true }).click();
  await expect(name).toHaveValue(" ");
  await expect.poll(() => name.evaluate(element => element.validity.customError)).toBe(true);
  await save.click();
  await expect(page.getByTestId("saved-instances")).toHaveText("0");
  await name.fill("Existing instance");
  await save.click();
  await expect(page.getByTestId("saved-instances")).toHaveText("1");
  await name.fill("  ");
  await save.click();
  await expect(page.getByTestId("saved-instances")).toHaveText("1");
});
