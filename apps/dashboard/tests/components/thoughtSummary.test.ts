import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import ThoughtSummary from '../../src/lib/components/session-chat/ThoughtSummary.svelte';
import type { SessionChatThoughtStep } from '../../src/lib/session-chat/sessionChat';

function step(overrides: Partial<SessionChatThoughtStep> = {}): SessionChatThoughtStep {
  return {
    id: 'thought-1',
    kind: 'thinking',
    title: 'Thinking',
    status: null,
    content: 'Inspecting the project.',
    occurredAt: '2026-06-11T00:00:00Z',
    ...overrides,
  };
}

test('idle thought summary collapses to a step count', () => {
  render(ThoughtSummary, {
    props: {
      steps: [
        step({ id: 'thought-1', kind: 'thinking', title: 'Thinking' }),
        step({ id: 'thought-2', kind: 'tool_call', title: 'bash' }),
        step({ id: 'thought-3', kind: 'tool_result', title: 'bash result' }),
      ],
      active: false,
    },
  });

  expect(screen.getByRole('button', { name: 'View thought details' })).toHaveTextContent('Thought for 3 steps');
  expect(screen.queryByText('bash')).not.toBeInTheDocument();
  expect(screen.queryByLabelText('Thinking in progress')).not.toBeInTheDocument();
});

test('busy thought summary shows recent steps with a loading icon', () => {
  render(ThoughtSummary, {
    props: {
      steps: [
        step({ id: 'thought-1', kind: 'thinking', title: 'Thinking', content: 'Planning changes' }),
        step({ id: 'thought-2', kind: 'tool_call', title: 'bash', content: 'rg ThoughtSummary' }),
        step({ id: 'thought-3', kind: 'tool_call', title: 'read', content: 'ThoughtSummary.svelte' }),
      ],
      active: true,
    },
  });

  expect(screen.queryByText('Thought for 3 steps')).not.toBeInTheDocument();
  expect(screen.getByLabelText('Thinking in progress')).toBeInTheDocument();
  expect(screen.queryByText('Planning changes')).not.toBeInTheDocument();
  expect(screen.getByText('bash')).toBeInTheDocument();
  expect(screen.getByText('read')).toBeInTheDocument();
});
