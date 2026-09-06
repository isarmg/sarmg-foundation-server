import {test,expect} from '@playwright/test';
test('configured instance workspace, header actions, names and consumer override',async({page})=>{
  const session={authenticated:true,user_id:'A'.repeat(43),username:'admin',role:'admin',csrf_token:'A'.repeat(43)};
  await page.route('**/api/v2/auth/session',route=>route.fulfill({json:session}));
  await page.goto('/#workspace');
  const actions=page.getByRole('group',{name:"Global actions"});
  await expect(actions.getByRole('button')).toHaveCount(5);
  await expect(page.getByRole('banner').locator('.sarmg-product-identity')).toBeVisible();
  await expect(page.getByRole('banner').locator('.sarmg-product-identity')).toHaveText('Foundation acceptance');
  await expect(page.locator('.sarmg-product-identity small')).toHaveCount(0);
  await expect(page.getByRole('button',{name:/诊断|Diagnostics/})).toHaveCount(0);
  for(const width of [1280,320]){
    await page.setViewportSize({width,height:800});
    const measurements=await page.locator('header .sarmg-header-navigation a, header .sarmg-header-actions button').evaluateAll(nodes=>nodes.map(node=>{const r=node.getBoundingClientRect(),s=getComputedStyle(node);return{y:r.y,height:r.height,size:s.fontSize};}));
    expect(measurements.length).toBe(8);
    const originalSize=await page.locator('body').evaluate(node=>getComputedStyle(node).fontSize);
    expect(measurements.every(value=>Math.abs(value.y-measurements[0].y)<1&&value.height===44&&value.size===originalSize)).toBe(true);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    for(const svg of await actions.locator('svg').all())expect((await svg.boundingBox()).height).toBe(parseFloat(originalSize));
  }
  await expect(page.getByRole('complementary').getByRole('button')).toHaveText(['Alpha','第二实例']);
  await page.getByRole('button',{name:"Select instance 第二实例"}).click();
  await expect(page.getByRole('heading',{name:'Selected 2'})).toBeVisible();
  await actions.getByRole('button',{name:"Refresh",exact:true}).click();
  await expect(page.getByText('Revision 1')).toBeVisible();
  await actions.getByRole('button',{name:"Create instance",exact:true}).click();
  const input=page.getByLabel('Instance name');
  for(const character of ['a','中','あ','😀']){
    await input.fill(character.repeat(32));expect(await input.evaluate(node=>node.checkValidity())).toBe(true);
    await input.fill(character.repeat(33));expect(await input.evaluate(node=>node.checkValidity())).toBe(false);
  }
  await page.keyboard.press('Escape');
  await expect(actions.getByRole('button',{name:"Create instance",exact:true})).toBeFocused();
  await page.goto('/?workspace=custom#workspace');
  await expect(page.locator('html')).toHaveAttribute('data-sarmg-appearance','custom-brand');
  await expect(page.locator('.sarmg-custom-workspace')).toBeVisible();
  await expect(page.locator('html')).toHaveAttribute('data-sarmg-selection','custom');
});
