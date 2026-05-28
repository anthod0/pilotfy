import assert from 'node:assert/strict';
import test from 'node:test';
import { chatAutoScrollKey, scrollToBottom } from './autoScroll.ts';

const message = (overrides = {}) => ({
  id: 'turn-1:assistant',
  turnId: 'turn-1',
  role: 'assistant',
  content: 'Waiting…',
  status: 'pending',
  createdAt: '2026-01-01T00:00:00Z',
  ...overrides,
});

test('chat auto-scroll key changes when a message is appended or the latest agent output changes', () => {
  const pending = [message()];
  const completed = [message({ content: 'Done', status: 'sent' })];
  const withUserReply = [...completed, message({ id: 'turn-2:user', turnId: 'turn-2', role: 'user', content: 'next', status: 'sent' })];

  assert.notEqual(chatAutoScrollKey(pending), chatAutoScrollKey(completed));
  assert.notEqual(chatAutoScrollKey(completed), chatAutoScrollKey(withUserReply));
});

test('scrollToBottom moves the scroll container to its bottom edge', () => {
  const element = { scrollTop: 0, scrollHeight: 640 };

  scrollToBottom(element);

  assert.equal(element.scrollTop, 640);
});
