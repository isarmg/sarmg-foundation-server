import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { configureNativeWorkspace } from '@sarmg/admin-shell/native';
import { DEFAULT_WORKSPACE_CONFIG } from '@sarmg/admin-shell/workspace-config';

test('native consumers use public entry points without loading the React shell', () => {
  assert.equal(typeof configureNativeWorkspace, 'function');
  assert.equal(DEFAULT_WORKSPACE_CONFIG.appearance, 'content-blocks');
  for (const file of ['native-workspace.js', 'workspace-config.js']) {
    const source = readFileSync(new URL(`../dist/${file}`, import.meta.url), 'utf8');
    assert.doesNotMatch(source, /from ["'](?:react|react-dom|\.\/index)/);
  }
});
