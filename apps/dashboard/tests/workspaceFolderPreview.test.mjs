import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../src/pages/WorkspacesPage.svelte', import.meta.url), 'utf8');

test('registered workspace cards include a dedicated folder preview illustration', () => {
  assert.match(source, /workspace-folder-preview/, 'expected a folder preview wrapper in registered workspace cards');
  assert.match(source, /workspace-folder-tab/, 'expected a folder tab element so the preview reads as a folder');
  assert.match(source, /workspace-folder-body/, 'expected a folder body element so the preview reads as a folder');
});
