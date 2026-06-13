import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from 'vitest';

const messageResponse = readFileSync(
  resolve(__dirname, '../src/lib/components/ai-elements/message/message-response.svelte'),
  'utf8',
);

const messageContent = readFileSync(
  resolve(__dirname, '../src/lib/components/ai-elements/message/message-content.svelte'),
  'utf8',
);

test('keeps markdown tables horizontally scrollable within the assistant response', () => {
  expect(messageContent).toContain('min-w-0');
  expect(messageResponse).toContain('[&_table]:block');
  expect(messageResponse).toContain('[&_table]:max-w-full');
  expect(messageResponse).toContain('[&_table]:overflow-x-auto');
});
