import {test,expect} from '@playwright/test';
test('configured admin workspace, header actions, names and consumer override',async({page})=>{
  const session={authenticated:true,user_id:'A'.repeat(43),username:'admin',role:'admin',csrf_token:'A'.repeat(43)};
  await page.route('**/api/v1/auth/session',route=>route.fulfill({json:session}));
  await page.goto('/#workspace');
  const actions=page.getByRole('group',{name:"Global actions"});
  await expect(actions.getByRole('button')).toHaveCount(6);
  await expect(page.getByRole('banner').locator('.xcss-product-identity')).toBeVisible();
  await expect(page.getByRole('banner').locator('.xcss-product-identity')).toHaveText('Foundation acceptance');
  await expect(page.locator('.xcss-product-identity small')).toHaveCount(0);
  await expect(page.getByRole('button',{name:/诊断|Diagnostics/})).toHaveCount(0);
  for(const width of [1280,320]){
    await page.setViewportSize({width,height:800});
    const measurements=await page.locator('header .xcss-header-navigation a, header .xcss-header-actions button').evaluateAll(nodes=>nodes.map(node=>{const r=node.getBoundingClientRect(),s=getComputedStyle(node);return{kind:node.closest('.xcss-header-actions')?'action':'menu',height:r.height,size:s.fontSize};}));
    expect(measurements.length).toBe(8);
    const originalSize=await page.locator('body').evaluate(node=>getComputedStyle(node).fontSize);
    expect(measurements.every(value=>value.height===(value.kind==='action'?44:30)&&value.size===originalSize)).toBe(true);
    expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
    for(const svg of await actions.locator('svg').all())expect((await svg.boundingBox()).height).toBe(parseFloat(originalSize));
  }
  await expect(page.getByRole('heading',{name:'Full-width workspace'})).toBeVisible();
  await expect(page.getByRole('complementary')).toHaveCount(0);
  await expect(page.locator('.xcss-instance-sidebar, .xcss-instance-workspace')).toHaveCount(0);
  await actions.getByRole('button',{name:"Refresh",exact:true}).click();
  await expect(page.getByText('Revision 1')).toBeVisible();
  await actions.getByRole('button',{name:"Create instance",exact:true}).click();
  const input=page.getByLabel('Instance name');
  for(const character of ['a','中','あ','😀']){
    await input.fill(character.repeat(32));expect(await input.evaluate(node=>node.checkValidity())).toBe(true);
    await input.fill(character.repeat(33));expect(await input.evaluate(node=>node.checkValidity())).toBe(false);
    await input.fill('  '+character.repeat(32)+'  ');expect(await input.evaluate(node=>node.checkValidity())).toBe(true);
  }
  for(const value of ['   ','name\x80','name\x9f']){
    await input.fill(value);expect(await input.evaluate(node=>node.checkValidity())).toBe(false);
  }
  await input.fill('\ufeff'+'a'.repeat(32));expect(await input.evaluate(node=>node.checkValidity())).toBe(false);
  await input.fill('\ufeff'+'a'.repeat(31));expect(await input.evaluate(node=>node.checkValidity())).toBe(true);
  await page.keyboard.press('Escape');
  await expect(actions.getByRole('button',{name:"Create instance",exact:true})).toBeFocused();
  await page.goto('/?workspace=custom#workspace');
  await expect(page.locator('html')).toHaveAttribute('data-xcss-appearance','custom-brand');
  await expect(page.locator('.xcss-instance-sidebar, .xcss-instance-workspace')).toHaveCount(0);
  await expect(page.locator('html')).toHaveAttribute('data-xcss-selection','custom');
});

test('initial instance names obey the same validation as edited values',async({page})=>{
  const session={authenticated:true,user_id:'A'.repeat(43),username:'admin',role:'admin',csrf_token:'A'.repeat(43)};
  await page.route('**/api/v1/auth/session',route=>route.fulfill({json:session}));
  await page.goto('/?instanceName=%20%20%20#workspace');
  await page.getByRole('button',{name:'Create instance',exact:true}).click();
  const input=page.getByLabel('Instance name');
  expect(await input.evaluate(node=>node.checkValidity())).toBe(false);
  await input.fill('Valid instance');
  expect(await input.evaluate(node=>node.checkValidity())).toBe(true);
});
