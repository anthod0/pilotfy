import { render, screen } from '@testing-library/svelte';
import { expect, test } from 'vitest';
import SessionConversation from '../src/lib/components/session-chat/SessionConversation.svelte';
import type { SessionChatMessage } from '../src/lib/session-chat/sessionChat';

const messages: SessionChatMessage[] = [
  {
    id: 'message-1',
    role: 'user',
    content: 'Please inspect the repo.',
    status: 'sent',
  },
  {
    id: 'message-2',
    role: 'assistant',
    content: 'I will inspect it now.',
    status: 'sent',
  },
];

test('conversation renders messages without role headers', () => {
  render(SessionConversation, { props: { messages } });

  expect(screen.getByText('Please inspect the repo.')).toBeInTheDocument();
  expect(screen.getByText('I will inspect it now.')).toBeInTheDocument();
  expect(screen.queryByText('You')).not.toBeInTheDocument();
  expect(screen.queryByText('AI')).not.toBeInTheDocument();
});
