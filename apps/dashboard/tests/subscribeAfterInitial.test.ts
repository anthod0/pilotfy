import assert from 'node:assert/strict';
import test from 'node:test';
import { writable } from 'svelte/store';
import { subscribeAfterInitial } from '../src/stores/subscribeAfterInitial.ts';

test('subscribeAfterInitial ignores the immediate store emission and handles later changes', () => {
  const store = writable('initial');
  const seen: string[] = [];

  const unsubscribe = subscribeAfterInitial(store, (value) => {
    seen.push(value);
  });

  assert.deepEqual(seen, []);

  store.set('changed');
  assert.deepEqual(seen, ['changed']);

  unsubscribe();
  store.set('ignored');
  assert.deepEqual(seen, ['changed']);
});
