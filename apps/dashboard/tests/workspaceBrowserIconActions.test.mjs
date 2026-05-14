import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const source = readFileSync(new URL('../src/pages/WorkspacesPage.svelte', import.meta.url), 'utf8');

test('workspace browser uses a compact directory/action table', () => {
  assert.doesNotMatch(source, /<Table\.Head>Kind<\/Table\.Head>/, 'kind column header should be removed');
  assert.doesNotMatch(source, /<Table\.Head>Workspace<\/Table\.Head>/, 'workspace status column should be folded into the active action');
  assert.match(source, /<Table\.Head>Directory<\/Table\.Head>[\s\S]*<Table\.Head class="text-right">Action<\/Table\.Head>/, 'table should only expose directory and action columns');
  assert.match(source, /<FolderOpen class="size-4 shrink-0 text-muted-foreground"/, 'directory names should include a folder icon');
  assert.match(source, />Open[\s\S]*<FolderOpen class="size-4"/, 'open action should include text and an open icon');
  assert.match(source, /Active\s*\{#if entry\.is_workspace\}/, 'active action should include text');
  assert.match(source, /CheckCircle2 class="size-4 text-primary"/, 'active action should show a selected icon');
  assert.match(source, /Circle class="size-4 text-muted-foreground"/, 'active action should show an unselected icon');
});

test('activating a directory opens a confirmation dialog with an editable default name', () => {
  assert.match(source, /activateEntry/, 'active action should open the activation flow');
  assert.match(source, /registerWorkspaceName = entry\.name/, 'activation should prefill the directory name');
  assert.match(source, /bind:value=\{registerWorkspaceName\}/, 'confirmation dialog should let users edit the workspace name');
  assert.match(source, /name: registerWorkspaceName\.trim\(\) \|\| null/, 'clearing the name should preserve default registration behavior');
  assert.match(source, /Confirm workspace registration/, 'confirmation dialog should be present');
});
