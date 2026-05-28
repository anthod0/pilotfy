import { beforeEach, expect, test, vi } from 'vitest';
import { listAgentProfiles, listWorkspaceRootEntries, listWorkspaceRoots, listWorkspaces } from '../src/api/client';

beforeEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
});

function jsonResponse(data: unknown): Response {
  return new Response(JSON.stringify({ data }), {
    status: 200,
    headers: { 'Content-Type': 'application/json' },
  });
}

test('passes AbortSignal through settings-related read requests', async () => {
  const fetchMock = vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    if (url.endsWith('/workspaces')) return jsonResponse({ workspaces: [] });
    if (url.endsWith('/workspace-roots')) return jsonResponse({ roots: [] });
    if (url.includes('/workspace-roots/root-1/entries')) {
      return jsonResponse({ root_id: 'root-1', path: '', canonical_path: '/repo', parent_path: null, entries: [], warnings: [] });
    }
    if (url.endsWith('/agent-profiles')) return jsonResponse({ agent_profiles: [] });
    throw new Error(`Unexpected request: ${url}`);
  });
  vi.stubGlobal('fetch', fetchMock);

  const controller = new AbortController();

  await listWorkspaces({ signal: controller.signal });
  await listWorkspaceRoots({ signal: controller.signal });
  await listWorkspaceRootEntries('root-1', '', { signal: controller.signal });
  await listAgentProfiles(false, { signal: controller.signal });

  expect(fetchMock).toHaveBeenCalledTimes(4);
  for (const [, init] of fetchMock.mock.calls) {
    expect((init as RequestInit).signal).toBe(controller.signal);
  }
});
