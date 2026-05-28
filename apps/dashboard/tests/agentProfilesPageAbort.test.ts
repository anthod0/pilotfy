import { render } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import AgentProfilesPage from '../src/pages/AgentProfilesPage.svelte';
import type { AgentProfileView } from '../src/api/types';

const mocks = vi.hoisted(() => {
  function writableStore<T>(initial: T) {
    let value = initial;
    const subscribers = new Set<(value: T) => void>();
    return {
      subscribe(run: (value: T) => void) {
        subscribers.add(run);
        run(value);
        return () => subscribers.delete(run);
      },
      set(next: T) {
        value = next;
        for (const run of subscribers) run(value);
      },
    };
  }

  const agentProfiles = writableStore<AgentProfileView[]>([]);
  const agentProfilesLoading = writableStore(false);
  const agentProfilesError = writableStore<string | null>(null);

  return {
    agentProfiles,
    agentProfilesLoading,
    agentProfilesError,
    loadAgentProfiles: vi.fn(async () => undefined),
    listAgentProfileVersions: vi.fn(async () => [] as AgentProfileView[]),
    createAgentProfile: vi.fn(),
    createAgentProfileVersion: vi.fn(),
    deleteAgentProfile: vi.fn(),
    deleteAgentProfileVersion: vi.fn(),
    updateAgentProfileVersion: vi.fn(),
  };
});

vi.mock('../src/stores/agentProfiles', () => ({
  agentProfiles: mocks.agentProfiles,
  agentProfilesLoading: mocks.agentProfilesLoading,
  agentProfilesError: mocks.agentProfilesError,
  loadAgentProfiles: mocks.loadAgentProfiles,
}));

vi.mock('../src/api/client', () => ({
  listAgentProfileVersions: mocks.listAgentProfileVersions,
  createAgentProfile: mocks.createAgentProfile,
  createAgentProfileVersion: mocks.createAgentProfileVersion,
  deleteAgentProfile: mocks.deleteAgentProfile,
  deleteAgentProfileVersion: mocks.deleteAgentProfileVersion,
  updateAgentProfileVersion: mocks.updateAgentProfileVersion,
}));

beforeEach(() => {
  mocks.agentProfiles.set([]);
  mocks.agentProfilesLoading.set(false);
  mocks.agentProfilesError.set(null);
  vi.clearAllMocks();
});

test('aborts initial settings agent profile requests when the page unmounts', async () => {
  const { unmount } = render(AgentProfilesPage);

  await vi.waitFor(() => expect(mocks.loadAgentProfiles).toHaveBeenCalled());
  const [, options] = mocks.loadAgentProfiles.mock.calls[0] as [boolean, { signal?: AbortSignal } | undefined];

  expect(options?.signal).toBeInstanceOf(AbortSignal);
  expect(options?.signal?.aborted).toBe(false);

  unmount();

  expect(options?.signal?.aborted).toBe(true);
});
