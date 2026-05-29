import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import ChatPage from '../src/pages/ChatPage.svelte';

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
      update(updater: (value: T) => T) {
        value = updater(value);
        for (const run of subscribers) run(value);
      },
    };
  }

  const session = {
    session_id: 'session-1',
    client_type: 'pi',
    handle: null,
    role: null,
    description: null,
    execution_profile_id: null,
    execution_profile_version: null,
    state: 'idle',
    current_turn_id: null,
    workspace_id: 'workspace-1',
    workspace: '/work/project',
    capabilities: {},
    created_at: '2026-05-14T00:00:00Z',
    updated_at: '2026-05-14T01:00:00Z',
    metadata: {},
  };

  return {
    navigate: vi.fn(),
    subscribeDashboardEvents: vi.fn(() => vi.fn()),
    sessions: writableStore([session]),
    sessionDetail: writableStore({ session, turns: [], inboxMessages: [], events: [], artifacts: [] }),
    sessionDetailLoading: writableStore(false),
    sessionDetailError: writableStore(null),
    sessionsError: writableStore(null),
    workspaces: writableStore([]),
    workspacesLoading: writableStore(false),
    workspacesError: writableStore(null),
    agentProfiles: writableStore([]),
    agentProfilesLoading: writableStore(false),
    agentProfilesError: writableStore(null),
    taskProposals: writableStore([]),
    taskProposalsLoading: writableStore(false),
    taskProposalsError: writableStore(null),
    loadSessions: vi.fn(async () => undefined),
    loadSessionDetail: vi.fn(async () => null),
    loadWorkspaces: vi.fn(async () => undefined),
    loadAgentProfiles: vi.fn(async () => undefined),
    loadTaskProposals: vi.fn(async () => undefined),
    createSession: vi.fn(),
    submitInboxMessage: vi.fn(),
    restartSession: vi.fn(),
    resumeSession: vi.fn(),
    terminateSession: vi.fn(),
    createDagTask: vi.fn(),
  };
});

vi.mock('svelte-mini-router', () => ({
  navigate: mocks.navigate,
  getPathParams: () => ({ sessionId: window.location.pathname.split('/').pop() }),
}));
vi.mock('svelte-sonner', () => ({ toast: { error: vi.fn() } }));
vi.mock('../src/services/eventStream', () => ({ subscribeDashboardEvents: mocks.subscribeDashboardEvents }));
vi.mock('../src/stores/sessions', () => ({
  sessions: mocks.sessions,
  sessionDetail: mocks.sessionDetail,
  sessionDetailLoading: mocks.sessionDetailLoading,
  sessionDetailError: mocks.sessionDetailError,
  sessionsError: mocks.sessionsError,
  loadSessions: mocks.loadSessions,
  loadSessionDetail: mocks.loadSessionDetail,
  createSession: mocks.createSession,
  submitInboxMessage: mocks.submitInboxMessage,
  restartSession: mocks.restartSession,
  resumeSession: mocks.resumeSession,
  terminateSession: mocks.terminateSession,
}));
vi.mock('../src/stores/workspaces', () => ({
  workspaces: mocks.workspaces,
  workspacesLoading: mocks.workspacesLoading,
  workspacesError: mocks.workspacesError,
  loadWorkspaces: mocks.loadWorkspaces,
}));
vi.mock('../src/stores/agentProfiles', async () => {
  const actual = await vi.importActual<typeof import('../src/stores/agentProfiles')>('../src/stores/agentProfiles');
  return {
    ...actual,
    agentProfiles: mocks.agentProfiles,
    agentProfilesLoading: mocks.agentProfilesLoading,
    agentProfilesError: mocks.agentProfilesError,
    loadAgentProfiles: mocks.loadAgentProfiles,
  };
});
vi.mock('../src/stores/tasks', () => ({
  taskProposals: mocks.taskProposals,
  taskProposalsLoading: mocks.taskProposalsLoading,
  taskProposalsError: mocks.taskProposalsError,
  loadTaskProposals: mocks.loadTaskProposals,
  createDagTask: mocks.createDagTask,
}));

beforeEach(() => {
  window.history.pushState({}, '', '/dashboard/chat/session-1');
  vi.clearAllMocks();
});

test('chat composer metadata and controls share one responsive toolbar', async () => {
  render(ChatPage);

  const toolbar = await screen.findByLabelText('Session status and controls');

  expect(within(toolbar).getByText('idle')).toBeInTheDocument();
  expect(within(toolbar).getByText('/work/project')).toBeInTheDocument();
  expect(within(toolbar).getByLabelText('Client: pi')).toBeInTheDocument();
  expect(within(toolbar).queryByText('Client: pi')).not.toBeInTheDocument();
  expect(within(toolbar).getByRole('button', { name: /exit session/i })).toBeInTheDocument();
  expect(within(toolbar).getByRole('button', { name: /advanced session controls/i })).toBeInTheDocument();
});

test('chat composer hides missing profile and handle metadata', async () => {
  render(ChatPage);

  await screen.findByLabelText('Session status and controls');

  expect(screen.queryByText(/^Profile:/)).not.toBeInTheDocument();
  expect(screen.queryByText(/^Handle:/)).not.toBeInTheDocument();
  expect(screen.queryByText('—')).not.toBeInTheDocument();
});
