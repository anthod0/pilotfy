import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { expect, test } from 'vitest';

const appCss = readFileSync(resolve(__dirname, '../src/app.css'), 'utf8');

test('defines subtle themed global scrollbar styling', () => {
  expect(appCss).toContain('scrollbar-width: thin');
  expect(appCss).toContain('scrollbar-color: var(--scrollbar-thumb) transparent');
  expect(appCss).toContain('::-webkit-scrollbar');
  expect(appCss).toContain('::-webkit-scrollbar-thumb:hover');
  expect(appCss).toContain('--scrollbar-thumb: color-mix(in oklch, var(--border) 75%, transparent)');
});
