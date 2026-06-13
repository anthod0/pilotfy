import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, test } from 'vitest';

const componentPath = (...parts: string[]) => resolve(__dirname, '../src/components/chat', ...parts);

describe('session metadata component boundaries', () => {
  test('composer dock uses viewport-specific session metadata components', () => {
    const source = readFileSync(componentPath('SessionComposerDock.svelte'), 'utf8');

    expect(source).toContain("import SessionMetadataDesktop from './SessionMetadataDesktop.svelte'");
    expect(source).toContain("import SessionMetadataMobile from './SessionMetadataMobile.svelte'");
    expect(source).not.toContain('SessionMetadataBadges');
  });
});
