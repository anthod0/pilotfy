import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import assert from 'node:assert/strict';

const sessionList = readFileSync(new URL('./SessionList.svelte', import.meta.url), 'utf8');
const createSessionForm = readFileSync(new URL('./CreateSessionForm.svelte', import.meta.url), 'utf8');
const apiTypes = readFileSync(new URL('../../api/types.ts', import.meta.url), 'utf8');

test('session views expose optional handle in web API types', () => {
  assert.match(apiTypes, /interface SessionView[\s\S]*handle:\s*string \| null;/);
  assert.match(apiTypes, /interface CreateSessionInput[\s\S]*handle\?:\s*string \| null;/);
});

test('session list presents handle as the primary session label when available', () => {
  assert.match(sessionList, /session\.handle \?\? session\.session_id/);
  assert.match(sessionList, /session\.handle\s*\?/);
});

test('create session form accepts an optional handle and submits trimmed non-empty values', () => {
  assert.match(createSessionForm, /let handle = '';/);
  assert.match(createSessionForm, /<label>Handle <input bind:value=\{handle\}/);
  assert.match(createSessionForm, /handle:\s*handle\.trim\(\) \|\| null/);
});
