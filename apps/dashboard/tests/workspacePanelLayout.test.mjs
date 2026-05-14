import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../src/pages/WorkspacesPage.svelte', import.meta.url), 'utf8');

test('workspace page removes the separate register current directory panel', () => {
  assert.doesNotMatch(source, /<Card\.Title>Register current directory<\/Card\.Title>/, 'separate register current directory panel should be removed');
  assert.doesNotMatch(source, /registerCurrentWorkspace/, 'registration should flow through Active actions instead of a separate panel action');
});

test('registered workspaces panel is shown above the root browser', () => {
  const registeredIndex = source.indexOf('<Card.Title>Active workspaces</Card.Title>');
  const browserIndex = source.indexOf('<Card.Title class="flex items-center gap-2"><FolderOpen class="size-5" /> Root browser</Card.Title>');

  assert.notEqual(registeredIndex, -1, 'active workspaces panel should exist');
  assert.notEqual(browserIndex, -1, 'root browser panel should exist');
  assert.ok(registeredIndex < browserIndex, 'active workspaces should appear above root browser');
});
