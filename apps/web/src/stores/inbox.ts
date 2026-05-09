import { writable } from 'svelte/store';
import { listInboxMessages, submitInboxMessage as apiSubmitInboxMessage } from '../api/client';
import type { InboxMessageView, SubmitInboxMessageInput } from '../api/types';

export const inboxMessages = writable<InboxMessageView[]>([]);
export const inboxLoading = writable(false);
export const inboxError = writable<string | null>(null);
export const lastSubmittedInboxMessage = writable<InboxMessageView | null>(null);

export async function loadInboxMessages(sessionId: string): Promise<void> {
  inboxLoading.set(true);
  inboxError.set(null);
  try {
    inboxMessages.set(await listInboxMessages(sessionId));
  } catch (error) {
    inboxError.set(error instanceof Error ? error.message : String(error));
  } finally {
    inboxLoading.set(false);
  }
}

export async function submitInboxMessage(sessionId: string, input: SubmitInboxMessageInput): Promise<InboxMessageView> {
  inboxError.set(null);
  const message = await apiSubmitInboxMessage(sessionId, input);
  lastSubmittedInboxMessage.set(message);
  await loadInboxMessages(sessionId);
  return message;
}
