import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { expect, test } from 'vitest';

const pageHeaderBadges = [
  ['src/pages/OverviewPage.svelte', 'Live External API'],
  ['src/pages/TasksPage.svelte', 'DAG-first workflow'],
  ['src/pages/ChatPage.svelte', 'Friendly session chat'],
  ['src/pages/SessionsPage.svelte', 'Advanced manual console'],
  ['src/pages/SettingsCommonPage.svelte', 'Settings / Common'],
  ['src/pages/WorkspacesPage.svelte', 'Configuration'],
  ['src/pages/AgentProfilesPage.svelte', 'Configuration'],
] as const;

test.each(pageHeaderBadges)('%s does not render the page-header capsule %s', (relativePath, label) => {
  const source = readFileSync(join(process.cwd(), relativePath), 'utf8');

  expect(source).not.toContain(`<Badge variant="secondary">${label}</Badge>`);
});
