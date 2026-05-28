import { render, screen, within } from '@testing-library/svelte';
import { beforeEach, expect, test, vi } from 'vitest';
import AppSidebarHost from './components/layout/AppSidebarHost.svelte';
import SettingsPage from '../src/pages/SettingsPage.svelte';

const mocks = vi.hoisted(() => ({
  navigate: vi.fn(),
  startEventStream: vi.fn(),
  stopEventStream: vi.fn(),
}));

vi.mock('svelte-mini-router', () => ({ navigate: mocks.navigate }));
vi.mock('../src/services/eventStream', () => ({
  startEventStream: mocks.startEventStream,
  stopEventStream: mocks.stopEventStream,
}));

beforeEach(() => {
  window.history.pushState({}, '', '/dashboard/overview');
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: vi.fn().mockImplementation((query: string) => ({
      matches: false,
      media: query,
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
});

test('sidebar shows workflow items and keeps settings in the footer', () => {
  render(AppSidebarHost);

  expect(screen.queryByText('Workflow')).not.toBeInTheDocument();
  expect(screen.queryByText('External API only')).not.toBeInTheDocument();

  const workflow = screen.getByText('Overview').closest('[data-slot="sidebar-group"]');
  expect(workflow).not.toBeNull();
  const footer = screen.getByText('Settings').closest('[data-slot="sidebar-footer"]');
  expect(footer).not.toBeNull();

  const workflowQueries = within(workflow as HTMLElement);
  expect(workflowQueries.getByText('Overview')).toBeInTheDocument();
  expect(workflowQueries.getByText('Tasks')).toBeInTheDocument();
  expect(workflowQueries.getByText('Chat')).toBeInTheDocument();
  expect(workflowQueries.queryByText('DAG Tasks')).not.toBeInTheDocument();
  expect(workflowQueries.queryByText('Session Console')).not.toBeInTheDocument();
  expect(workflowQueries.queryByText('Workspaces')).not.toBeInTheDocument();
  expect(workflowQueries.queryByText('Agent Profiles')).not.toBeInTheDocument();
  expect(workflowQueries.queryByText('Settings')).not.toBeInTheDocument();

  expect(within(footer as HTMLElement).getByText('Settings')).toBeInTheDocument();
});

test('sidebar only marks the current route as active', () => {
  window.history.pushState({}, '', '/dashboard/chat');

  render(AppSidebarHost);

  const overview = screen.getByText('Overview').closest('button');
  const tasks = screen.getByText('Tasks').closest('button');
  const chat = screen.getByText('Chat').closest('button');
  const settings = screen.getByText('Settings').closest('button');

  expect(overview).not.toBeNull();
  expect(tasks).not.toBeNull();
  expect(chat).not.toBeNull();
  expect(settings).not.toBeNull();

  expect(chat).toHaveAttribute('data-active', 'true');
  expect(overview).not.toHaveAttribute('data-active');
  expect(tasks).not.toHaveAttribute('data-active');
  expect(settings).not.toHaveAttribute('data-active');
});

test('settings page exposes administration links moved out of the sidebar', () => {
  render(SettingsPage);

  expect(screen.getByRole('link', { name: /workspaces/i })).toHaveAttribute('href', '/dashboard/workspaces');
  expect(screen.getByRole('link', { name: /agent profiles/i })).toHaveAttribute('href', '/dashboard/agent-profiles');
});
