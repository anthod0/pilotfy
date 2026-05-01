import { get } from 'svelte/store';
import { loadArtifacts } from '../stores/artifacts';
import { selectedSessionId } from '../stores/selection';
import { refreshSession } from '../stores/sessionDetail';
import { loadSessions } from '../stores/sessions';
import { loadTurns } from '../stores/turns';

type RefreshTask = () => Promise<void>;

function coalesce(delayMs: number, task: RefreshTask): () => void {
  let timer: ReturnType<typeof setTimeout> | null = null;
  let running = false;
  let rerun = false;

  async function run() {
    timer = null;
    if (running) {
      rerun = true;
      return;
    }
    running = true;
    try {
      await task();
    } finally {
      running = false;
      if (rerun) {
        rerun = false;
        timer = setTimeout(run, delayMs);
      }
    }
  }

  return () => {
    if (timer) clearTimeout(timer);
    timer = setTimeout(run, delayMs);
  };
}

export const refreshSelectedSession = coalesce(100, async () => {
  const id = get(selectedSessionId);
  if (id) await refreshSession(id);
});

export const refreshTurns = coalesce(150, async () => {
  const id = get(selectedSessionId);
  if (id) await loadTurns(id);
});

export const refreshSessionList = coalesce(250, loadSessions);

export const refreshArtifacts = coalesce(250, async () => {
  const id = get(selectedSessionId);
  if (id) await loadArtifacts(id);
});
