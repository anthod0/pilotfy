import { derived, get, writable } from 'svelte/store';
import { listEvents } from '../api/client';
import type { EventView } from '../api/types';

export const events = writable<EventView[]>([]);
export const eventsBySession = writable<Record<string, EventView[]>>({});
export const seenEventIdsBySession = writable<Record<string, Record<string, true>>>({});
export const lastEventIdBySession = writable<Record<string, string | null>>({});
export const eventsLoading = writable(false);
export const eventsError = writable<string | null>(null);

function sortEvents(items: EventView[]): EventView[] {
  return [...items].sort((a, b) => a.time.localeCompare(b.time));
}

function seedSessionEvents(sessionId: string, items: EventView[]): void {
  const sorted = sortEvents(items);
  const seen: Record<string, true> = {};
  for (const event of sorted) seen[event.event_id] = true;

  eventsBySession.update((state) => ({ ...state, [sessionId]: sorted }));
  seenEventIdsBySession.update((state) => ({ ...state, [sessionId]: seen }));
  lastEventIdBySession.update((state) => ({ ...state, [sessionId]: sorted.at(-1)?.event_id ?? null }));
  events.set(sorted);
}

export async function loadEvents(sessionId: string): Promise<void> {
  eventsLoading.set(true);
  eventsError.set(null);
  try {
    seedSessionEvents(sessionId, await listEvents(sessionId));
  } catch (error) {
    eventsError.set(error instanceof Error ? error.message : String(error));
  } finally {
    eventsLoading.set(false);
  }
}

export function showCachedEvents(sessionId: string): void {
  events.set(get(eventsBySession)[sessionId] ?? []);
}

export function appendEvent(event: EventView): boolean {
  const seenForSession = get(seenEventIdsBySession)[event.session_id] ?? {};
  if (seenForSession[event.event_id]) return false;

  seenEventIdsBySession.update((state) => ({
    ...state,
    [event.session_id]: { ...(state[event.session_id] ?? {}), [event.event_id]: true },
  }));

  let nextEvents: EventView[] = [];
  eventsBySession.update((state) => {
    nextEvents = sortEvents([...(state[event.session_id] ?? []), event]);
    return { ...state, [event.session_id]: nextEvents };
  });
  lastEventIdBySession.update((state) => ({ ...state, [event.session_id]: event.event_id }));
  events.set(nextEvents);
  return true;
}

export const latestEventOutput = derived(events, ($events) => {
  for (const event of [...$events].reverse()) {
    if (event.type !== 'turn.output' && event.type !== 'turn.completed') continue;
    const output = event.payload.output;
    if (output && typeof output === 'object' && 'summary' in output && typeof output.summary === 'string') {
      return output.summary;
    }
  }
  return null;
});
