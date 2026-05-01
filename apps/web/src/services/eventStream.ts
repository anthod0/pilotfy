import { get } from 'svelte/store';
import type { EventView } from '../api/types';
import { token } from '../stores/auth';
import { sseStatus, reconnectCount, lastConnectionError, streamedSessionId } from '../stores/connection';
import { appendEvent, lastEventIdBySession } from '../stores/events';
import { selectedSessionId } from '../stores/selection';
import { refreshArtifacts, refreshSelectedSession, refreshSessionList, refreshTurns } from './refreshCoordinator';

const API_BASE = '/external/v1';
const TERMINAL_TURN_EVENTS = new Set(['turn.completed', 'turn.failed', 'turn.interrupted', 'turn.cancelled']);
const SESSION_STATE_EVENTS = new Set(['session.ready', 'session.started', 'session.exited', 'session.error']);

let controller: AbortController | null = null;
let currentSessionId: string | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let generation = 0;

function clearReconnectTimer(): void {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}

export function stopEventStream(): void {
  generation += 1;
  clearReconnectTimer();
  controller?.abort();
  controller = null;
  currentSessionId = null;
  streamedSessionId.set(null);
  sseStatus.set('closed');
}

export function startEventStream(sessionId: string): void {
  if (currentSessionId === sessionId) return;
  stopEventStream();
  reconnectCount.set(0);
  lastConnectionError.set(null);
  currentSessionId = sessionId;
  streamedSessionId.set(sessionId);
  void connect(sessionId, generation);
}

async function connect(sessionId: string, streamGeneration: number): Promise<void> {
  const bearer = get(token).trim();
  if (!bearer) {
    sseStatus.set('idle');
    lastConnectionError.set('Set an API token to open the event stream.');
    return;
  }

  controller = new AbortController();
  sseStatus.set(get(reconnectCount) > 0 ? 'reconnecting' : 'connecting');
  lastConnectionError.set(null);

  try {
    const after = get(lastEventIdBySession)[sessionId];
    const query = after ? `?after=${encodeURIComponent(after)}` : '';
    const response = await fetch(`${API_BASE}/sessions/${encodeURIComponent(sessionId)}/events/stream${query}`, {
      headers: { Authorization: `Bearer ${bearer}` },
      signal: controller.signal,
    });

    if (!response.ok || !response.body) throw new Error(`Event stream failed: ${response.status} ${response.statusText}`);
    sseStatus.set('open');
    await readSse(response.body, (event) => handleEvent(event, sessionId));

    if (streamGeneration === generation && get(selectedSessionId) === sessionId) scheduleReconnect(sessionId, streamGeneration);
  } catch (error) {
    if (controller?.signal.aborted || streamGeneration !== generation) return;
    lastConnectionError.set(error instanceof Error ? error.message : String(error));
    sseStatus.set('error');
    scheduleReconnect(sessionId, streamGeneration);
  }
}

function scheduleReconnect(sessionId: string, streamGeneration: number): void {
  if (get(selectedSessionId) !== sessionId || streamGeneration !== generation) return;
  reconnectCount.update((count) => count + 1);
  const delay = Math.min(1000 + get(reconnectCount) * 500, 5000);
  sseStatus.set('reconnecting');
  clearReconnectTimer();
  reconnectTimer = setTimeout(() => connect(sessionId, streamGeneration), delay);
}

async function readSse(body: ReadableStream<Uint8Array>, onEvent: (event: EventView) => void): Promise<void> {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    let boundary = buffer.search(/\r?\n\r?\n/);
    while (boundary !== -1) {
      const frame = buffer.slice(0, boundary);
      buffer = buffer.slice(buffer[boundary] === '\r' ? boundary + 4 : boundary + 2);
      parseFrame(frame, onEvent);
      boundary = buffer.search(/\r?\n\r?\n/);
    }
  }
}

function parseFrame(frame: string, onEvent: (event: EventView) => void): void {
  const dataLines: string[] = [];
  for (const line of frame.split(/\r?\n/)) {
    if (line.startsWith('data:')) dataLines.push(line.slice(5).trimStart());
  }
  if (!dataLines.length) return;
  try {
    onEvent(JSON.parse(dataLines.join('\n')) as EventView);
  } catch (error) {
    lastConnectionError.set(error instanceof Error ? error.message : String(error));
  }
}

function handleEvent(event: EventView, sessionId: string): void {
  if (event.session_id !== sessionId) return;
  const accepted = appendEvent(event);
  if (!accepted) return;

  if (event.type === 'turn.output') {
    refreshTurns();
  } else if (TERMINAL_TURN_EVENTS.has(event.type)) {
    refreshSelectedSession();
    refreshTurns();
  } else if (SESSION_STATE_EVENTS.has(event.type)) {
    refreshSelectedSession();
    refreshSessionList();
  } else if (event.type.startsWith('artifact.')) {
    refreshArtifacts();
  }
}
