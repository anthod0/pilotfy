import { writable } from 'svelte/store';
import { discoverArtifacts, getArtifactContent, listArtifacts } from '../api/client';
import type { ArtifactContent, ArtifactView, TaskDagView, TaskView } from '../api/types';

export interface TaskArtifactItem extends ArtifactView {
  source_session_id: string;
}

export const taskArtifacts = writable<TaskArtifactItem[]>([]);
export const taskArtifactsLoading = writable(false);
export const taskArtifactsError = writable<string | null>(null);
export const selectedArtifactContent = writable<ArtifactContent | null>(null);
export const artifactContentLoading = writable(false);
export const artifactContentError = writable<string | null>(null);

function taskSessionIds(task: TaskView | null, dag: TaskDagView | null): string[] {
  const ids = new Set<string>();
  if (task?.session_id) ids.add(task.session_id);
  for (const run of dag?.runs ?? []) if (run.session_id) ids.add(run.session_id);
  for (const item of dag?.work_items ?? []) if (item.runtime?.session_id) ids.add(item.runtime.session_id);
  return [...ids];
}

export async function loadTaskArtifacts(task: TaskView | null, dag: TaskDagView | null): Promise<void> {
  const sessionIds = taskSessionIds(task, dag);
  taskArtifactsLoading.set(true);
  taskArtifactsError.set(null);
  try {
    const artifactGroups = await Promise.all(sessionIds.map(async (sessionId) => {
      const artifacts = await listArtifacts(sessionId);
      return artifacts.map((artifact) => ({ ...artifact, source_session_id: sessionId }));
    }));
    taskArtifacts.set(artifactGroups.flat().sort((a, b) => b.created_at.localeCompare(a.created_at)));
  } catch (error) {
    taskArtifacts.set([]);
    taskArtifactsError.set(error instanceof Error ? error.message : String(error));
  } finally {
    taskArtifactsLoading.set(false);
  }
}

export async function discoverTaskArtifacts(task: TaskView | null, dag: TaskDagView | null): Promise<void> {
  const sessionIds = taskSessionIds(task, dag);
  taskArtifactsLoading.set(true);
  taskArtifactsError.set(null);
  try {
    const artifactGroups = await Promise.all(sessionIds.map(async (sessionId) => {
      const artifacts = await discoverArtifacts(sessionId);
      return artifacts.map((artifact) => ({ ...artifact, source_session_id: sessionId }));
    }));
    taskArtifacts.set(artifactGroups.flat().sort((a, b) => b.created_at.localeCompare(a.created_at)));
  } catch (error) {
    taskArtifacts.set([]);
    taskArtifactsError.set(error instanceof Error ? error.message : String(error));
  } finally {
    taskArtifactsLoading.set(false);
  }
}

export async function loadArtifactContent(artifactId: string): Promise<void> {
  artifactContentLoading.set(true);
  artifactContentError.set(null);
  selectedArtifactContent.set(null);
  try {
    selectedArtifactContent.set(await getArtifactContent(artifactId));
  } catch (error) {
    artifactContentError.set(error instanceof Error ? error.message : String(error));
  } finally {
    artifactContentLoading.set(false);
  }
}
