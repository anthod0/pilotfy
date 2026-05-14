import { writable } from 'svelte/store';
import { getSession, listEvents, listTurns } from '../api/client';
import type { EventView, SessionView, TaskDagView, TaskView, TurnView } from '../api/types';

export interface TaskSessionDetail {
  session: SessionView;
  turns: TurnView[];
  events: EventView[];
  referencedBy: string[];
}

export const taskSessions = writable<TaskSessionDetail[]>([]);
export const taskSessionsLoading = writable(false);
export const taskSessionsError = writable<string | null>(null);

function taskSessionRefs(task: TaskView | null, dag: TaskDagView | null): Map<string, Set<string>> {
  const refs = new Map<string, Set<string>>();
  const add = (sessionId: string | null | undefined, ref: string) => {
    if (!sessionId) return;
    const existing = refs.get(sessionId) ?? new Set<string>();
    existing.add(ref);
    refs.set(sessionId, existing);
  };

  add(task?.session_id, 'task');
  for (const run of dag?.runs ?? []) add(run.session_id, `run ${run.run_id}`);
  for (const item of dag?.work_items ?? []) add(item.runtime?.session_id, `work item ${item.work_item_id}`);
  for (const signal of dag?.signals ?? []) add(signal.source_session_id, `signal ${signal.signal_id}`);
  return refs;
}

export async function loadTaskSessions(task: TaskView | null, dag: TaskDagView | null): Promise<void> {
  const refs = taskSessionRefs(task, dag);
  taskSessionsLoading.set(true);
  taskSessionsError.set(null);
  try {
    const details = await Promise.all([...refs.entries()].map(async ([sessionId, referencedBy]) => {
      const [session, turns, events] = await Promise.all([getSession(sessionId), listTurns(sessionId), listEvents(sessionId)]);
      return { session, turns, events, referencedBy: [...referencedBy] } satisfies TaskSessionDetail;
    }));
    taskSessions.set(details.sort((a, b) => b.session.updated_at.localeCompare(a.session.updated_at)));
  } catch (error) {
    taskSessions.set([]);
    taskSessionsError.set(error instanceof Error ? error.message : String(error));
  } finally {
    taskSessionsLoading.set(false);
  }
}
