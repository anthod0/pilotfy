import { render, screen } from '@testing-library/svelte';
import { expect, test, vi } from 'vitest';
import ThoughtSummaryCollapsed from '../../src/lib/components/session-chat/ThoughtSummaryCollapsed.svelte';

test('collapsed thought summary fills container without icon, background, border, or shadow and uses smaller title text', () => {
  const { container } = render(ThoughtSummaryCollapsed, {
    props: {
      step: {
        id: 'thought-1',
        kind: 'thinking',
        title: 'Thinking',
        status: 'completed',
        content: 'Inspecting the project.',
        occurredAt: '2026-06-11T00:00:00Z',
      },
      stepCount: 3,
      active: false,
      onOpen: vi.fn(),
    },
  });

  const button = screen.getByRole('button', { name: 'View thought details' });
  expect(button).toHaveClass('w-full');
  expect(button).not.toHaveClass('bg-muted/20');
  expect(button).not.toHaveClass('hover:bg-muted/35');
  expect(button).not.toHaveClass('border');
  expect(button).not.toHaveClass('border-border/70');
  expect(button).not.toHaveClass('hover:border-border');
  expect(button).not.toHaveClass('shadow-sm');
  expect(screen.queryByText('3 steps')).not.toBeInTheDocument();
  expect(screen.queryByText('completed')).not.toBeInTheDocument();
  expect(screen.getByLabelText('completed')).toBeInTheDocument();
  expect(container.querySelector('svg')).toBeInTheDocument();
  expect(screen.getByText('Thinking')).toHaveClass('text-sm');
  expect(screen.getByText('Thinking')).not.toHaveClass('text-base');
  expect(screen.getByText('Inspecting the project.')).toHaveClass('line-clamp-1');
  expect(screen.getByText('Inspecting the project.')).not.toHaveClass('line-clamp-2');
});
