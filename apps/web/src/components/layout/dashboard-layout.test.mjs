import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const sidebar = readFileSync(new URL('./Sidebar.svelte', import.meta.url), 'utf8');
const appShell = readFileSync(new URL('./AppShell.svelte', import.meta.url), 'utf8');
const statusBar = readFileSync(new URL('./StatusBar.svelte', import.meta.url), 'utf8');
const globalCss = readFileSync(new URL('../../styles/global.css', import.meta.url), 'utf8');

test('dashboard uses session-first layout without task/planner panels', () => {
  assert.match(sidebar, /import CreateSessionForm from/);
  assert.match(sidebar, /<CreateSessionForm \/>/);
  assert.doesNotMatch(sidebar, /CreateTaskForm|TaskList|Compatibility: create session directly/);
  assert.doesNotMatch(sidebar, /<details class="panel">/);

  assert.doesNotMatch(appShell, /TaskDetail|tasks\/TaskDetail/);
});

test('status bar keeps the API token input interactive under long status text', () => {
  assert.match(statusBar, /<label class="token-field">/);
  assert.match(statusBar, /<input[^>]+aria-label="External API token"/);
  assert.match(globalCss, /\.token-field\s*\{[^}]*flex:\s*0\s+0\s+min\(18rem,\s*100%\)/s);
  assert.match(globalCss, /\.status-stack\s*\{[^}]*min-width:\s*0/s);
  assert.match(globalCss, /\.status-stack\s+(?:span|small)\s*\{[^}]*overflow-wrap:\s*anywhere/s);
});

test('status bar API token input uses readable text color instead of inheriting the dark header color', () => {
  assert.match(globalCss, /\.token-field input\s*\{[^}]*color:\s*var\(--text\)/s);
});
