import test from 'node:test';
import assert from 'node:assert/strict';
import { createElement } from 'react';
import { renderToStaticMarkup } from 'react-dom/server';
import { DEFAULT_WORKSPACE_CONFIG, resolveWorkspaceConfig, validInstanceName, InstanceWorkspace, InstanceNameField, WorkspaceIcon } from '../dist/index.js';
test('default workspace policy and explicit consumer customization',()=>{
  assert.equal(DEFAULT_WORKSPACE_CONFIG.instanceNameMaxCharacters,32);
  assert.equal(DEFAULT_WORKSPACE_CONFIG.selection,'underline');
  assert.equal(DEFAULT_WORKSPACE_CONFIG.headerControls,'icons');
  assert.equal(DEFAULT_WORKSPACE_CONFIG.diagnostics,false);
  assert.equal(DEFAULT_WORKSPACE_CONFIG.showVersion,false);
  assert.equal(DEFAULT_WORKSPACE_CONFIG.headerIconSize,'1em');
  for(const input of [{diagnostics:true},{showVersion:true},{navigationPlacement:'sidebar'},{headerIconSize:'22px'}])assert.throws(()=>resolveWorkspaceConfig(input));
  assert.equal(resolveWorkspaceConfig({appearance:'custom-brand',layout:'custom'}).appearance,'custom-brand');
  assert.equal(DEFAULT_WORKSPACE_CONFIG.appearance,'content-blocks');
  for(const input of [{instanceNameMaxCharacters:33},{instanceNameMaxCharacters:0},{appearance:'../invalid'}])assert.throws(()=>resolveWorkspaceConfig(input));
});
test('instance names count Unicode scalar values, reject controls and blank names',()=>{
  for(const character of ['a','中','あ','😀']) { assert.equal(validInstanceName(character.repeat(32)),true); assert.equal(validInstanceName(character.repeat(33)),false); }
  for(const name of ['','   ','name\ncontrol','name\x7f'])assert.equal(validInstanceName(name),false);
});
test('shared workspace has name-only sidebar and product-owned details',()=>{
  const html=renderToStaticMarkup(createElement(InstanceWorkspace,{instances:[{id:'1',name:'Example'}],selected:'1',select(){}},'Settings'));
  assert.match(html,/aria-pressed="true"/);assert.match(html,/<span>Example<\/span>/);assert.match(html,/Instance details and settings/);
  const field=renderToStaticMarkup(createElement(InstanceNameField));assert.doesNotMatch(field,/maxlength/i);assert.ok(field.includes('{1,32}'));
  assert.match(renderToStaticMarkup(createElement(WorkspaceIcon,{name:'create'})),/aria-hidden="true"/);
});
test('empty or irrelevant instance sidebars do not reserve a blank column',()=>{
  for(const props of [{instances:[]},{instances:[{id:'1',name:'Example'}],showSidebar:false}]){
    const html=renderToStaticMarkup(createElement(InstanceWorkspace,{...props,select(){}},'Full width content'));
    assert.doesNotMatch(html,/<aside/);assert.match(html,/sarmg-instance-workspace--full/);assert.match(html,/Full width content/);
  }
});
