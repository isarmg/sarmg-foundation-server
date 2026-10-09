import { test, expect } from "@playwright/test";
import AxeBuilder from "@axe-core/playwright";
test.setTimeout(90_000);

for (const colorScheme of ["light", "dark"]) for (const width of [360, 1280]) {
  test(`editable date ranges apply on Enter and mark only invalid numbers (${colorScheme}, ${width})`, async ({ page }) => {
    await page.emulateMedia({ colorScheme }); await page.setViewportSize({ width, height: 740 });
    await page.route("**/api/v1/**", route => route.fulfill({ json: { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) } }));
    await page.goto("/?lang=en#dates");
    await expect(page.locator("html")).toHaveAttribute("data-xcss-fonts", "ready", { timeout: 30_000 });
    const year = page.getByRole("textbox", { name: "Start date Year", exact: true });
    const month = page.getByRole("textbox", { name: "Start date Month", exact: true });
    const day = page.getByRole("textbox", { name: "Start date Day", exact: true });
    const applied = page.getByTestId("applied-range");
    await expect(page.getByRole("textbox")).toHaveCount(6);
    await day.fill("30");
    await expect(day).toHaveAttribute("aria-invalid", "true");
    await expect(page.getByRole("alert")).toHaveText("Enter a valid year, month and day.");
    const colors = await page.locator("#fixture-date").evaluate(root => ({ input: getComputedStyle(root.querySelector('[aria-invalid="true"]')).color, separators: [...root.querySelectorAll('.xcss-date-range-separator')].map(node => getComputedStyle(node).color) }));
    expect(colors.separators.every(color => color !== colors.input)).toBe(true);
    await day.press("Enter"); await expect(applied).toHaveText("2022-02-01/2023-02-02 (0)");
    await year.fill("2020"); await month.fill("2"); await day.fill("29");
    await expect(page.getByRole("alert")).toHaveCount(0);
    await expect(applied).toHaveText("2022-02-01/2023-02-02 (0)");
    await day.dispatchEvent("keydown", { key: "Enter", isComposing: true });
    await expect(applied).toHaveText("2022-02-01/2023-02-02 (0)");
    await day.press("Enter"); await expect(applied).toHaveText("2020-02-29/2023-02-02 (1)");
    await year.fill("2024"); await day.press("Enter");
    await expect(page.getByRole("alert")).toHaveText("The end date must not precede the start date.");
    await expect(page.locator('input[aria-invalid="true"]')).toHaveCount(6);
    await expect(applied).toHaveText("2020-02-29/2023-02-02 (1)");
    await year.fill("no"); await expect(year).toHaveAttribute("aria-invalid", "true");
    await year.fill("2000"); await day.press("Enter");
    await expect(applied).toHaveText("2000-02-29/2023-02-02 (2)");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    expect((await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa", "wcag21aa"]).analyze()).violations).toEqual([]);
  });
}
