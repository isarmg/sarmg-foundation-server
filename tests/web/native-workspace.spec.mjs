import {test,expect} from '@playwright/test';
import {readFile} from 'node:fs/promises';
test('native consumers share icons and keep product callbacks and theme behavior',async({page})=>{
  const config=await readFile(new URL('../../packages/admin-shell/dist/workspace-config.js',import.meta.url),'utf8');
  const native=await readFile(new URL('../../packages/admin-shell/dist/native-workspace.js',import.meta.url),'utf8');
  await page.goto('/');
  await page.setContent('<header id="head"><div id="actions"><button id="create">Create</button><button id="logout"><span>admin</span></button></div></header><main id="content">Files</main>');
  await page.addScriptTag({type:'module',content:config+'\n'+native.replace(/^import .*?;\n/,'')+'\nconst logout=document.getElementById("logout"); logout.addEventListener("click",()=>document.body.dataset.loggedOut="true"); const create=document.getElementById("create"); create.textContent=""; create.addEventListener("click",()=>document.body.dataset.created="true"); configureNativeWorkspace({header:document.getElementById("head"),content:document.getElementById("content"),actions:document.getElementById("actions"),create,logout,refresh:()=>document.body.dataset.refreshed="true",instanceName:"Shared root",instanceHref:"/"});'});
  const actions=page.getByRole('group',{name:'全局操作'});await expect(actions.getByRole('button')).toHaveCount(4);
  await actions.getByRole('button',{name:'新建实例'}).click();await expect(page.locator('body')).toHaveAttribute('data-created','true');
  await actions.getByRole('button',{name:'刷新',exact:true}).click();await expect(page.locator('body')).toHaveAttribute('data-refreshed','true');
  const previous=await page.locator('html').getAttribute('data-theme');await actions.getByRole('button',{name:/切换到.*模式/}).click();await expect(page.locator('html')).not.toHaveAttribute('data-theme',previous);
  await expect(page.getByRole('complementary')).toHaveText('Shared root');
  await actions.getByRole('button',{name:'退出',exact:true}).click();await expect(page.locator('body')).toHaveAttribute('data-logged-out','true');
});
