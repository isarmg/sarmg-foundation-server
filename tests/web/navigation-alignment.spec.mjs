import { test, expect } from "@playwright/test";

test("wrapped primary and secondary menus align every row's first text with the menu edge", async ({ page }) => {
  const session = { authenticated: true, user_id: "A".repeat(43), username: "admin", role: "admin", csrf_token: "A".repeat(43) };
  await page.route("**/api/v1/**", route => route.fulfill({ json: session }));
  await page.goto("/?loginLanding=1#workspace");
  await expect(page.getByRole("heading", { name: "Full-width workspace" })).toBeVisible();
  await page.locator(".xcss-header-navigation").evaluate(nav => {
    for (const label of ["实例列表", "详细信息", "日志", "CPU", "内存", "设置"]) {
      const link = document.createElement("a");
      link.href = "#workspace";
      link.textContent = label;
      nav.append(link);
    }
  });
  await page.locator(".xcss-shell-main > .xcss-content-stack").evaluate(stack => {
    for (const labels of [
      ["历史趋势", "实例概览", "CPU", "内存", "网络硬件", "硬件传感器", "磁盘健康", "网络接口", "磁盘", "温度传感器", "显卡", "采集能力与诊断", "监控状态"],
      ["实例概览", "概况", "输入", "音频/视频", "网络", "配置文件", "高级", "NVIDIA NVENC 编码器", "Moonlight 配对", "预览变更"],
    ]) {
      const nav = document.createElement("nav");
      nav.className = "xcss-secondary-navigation";
      nav.dataset.alignmentFixture = "true";
      for (const label of labels) {
        const button = document.createElement("button");
        button.type = "button";
        button.className = "xcss-button";
        button.textContent = label;
        nav.append(button);
      }
      stack.append(nav);
    }
  });

  for (const width of [1280, 960, 768, 540, 360, 320]) {
    await page.setViewportSize({ width, height: 900 });
    const menus = await page.locator(".xcss-header-navigation, [data-alignment-fixture]").evaluateAll(navs => navs.map(nav => {
      const edge = nav.getBoundingClientRect().left;
      const rows = [];
      const items = [...nav.children].map(item => {
        const rect = item.getBoundingClientRect();
        const walker = document.createTreeWalker(item, NodeFilter.SHOW_TEXT);
        let text;
        while ((text = walker.nextNode()) && !text.textContent.trim()) {}
        if (!text) throw new Error("Menu fixture has an empty label");
        const start = text.textContent.search(/\S/);
        const range = document.createRange();
        range.setStart(text, start);
        range.setEnd(text, start + 1);
        const x = range.getBoundingClientRect().left;
        let row = rows.find(value => Math.abs(value.y - rect.top) < 0.5);
        if (!row) { row = { y: rect.top, items: [] }; rows.push(row); }
        row.items.push({ left: rect.left, x });
        return { label: item.textContent, width: rect.width, textOffset: x - rect.left };
      });
      return {
        primary: nav.classList.contains("xcss-header-navigation"),
        rowGap: getComputedStyle(nav).rowGap,
        rowStarts: rows.map(row => row.items.sort((a, b) => a.left - b.left)[0].x - edge),
        items,
      };
    }));
    expect(menus).toHaveLength(3);
    for (const menu of menus) {
      expect(menu.rowGap).toBe("8px");
      if (width <= 540) expect(menu.rowStarts.length).toBeGreaterThan(1);
      for (const start of menu.rowStarts) expect(Math.abs(start)).toBeLessThan(0.5);
      for (const item of menu.items) {
        expect(item.width).toBeGreaterThanOrEqual(44);
        expect(Math.abs(item.textOffset), item.label).toBeLessThan(0.5);
      }
    }
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  }
});
