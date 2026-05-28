import { writable } from 'svelte/store';
import {
  listWorkspaceRootEntries,
  deleteWorkspace as apiDeleteWorkspace,
  listWorkspaceRoots,
  listWorkspaces,
  registerWorkspace as apiRegisterWorkspace,
  renameWorkspace as apiRenameWorkspace,
  type ReadRequestOptions,
} from '../api/client';
import type {
  RegisterWorkspaceInput,
  RenameWorkspaceInput,
  WorkspaceDirectoryListingView,
  WorkspaceRootView,
  WorkspaceView,
} from '../api/types';

export const workspaces = writable<WorkspaceView[]>([]);
export const workspacesLoading = writable(false);
export const workspacesError = writable<string | null>(null);
export const workspaceRoots = writable<WorkspaceRootView[]>([]);

function isAbortError(error: unknown): boolean {
  return error instanceof DOMException && error.name === 'AbortError';
}

export async function loadWorkspaces(options: ReadRequestOptions = {}): Promise<void> {
  workspacesLoading.set(true);
  workspacesError.set(null);
  try {
    workspaces.set(await listWorkspaces(options));
  } catch (error) {
    if (!isAbortError(error)) workspacesError.set(error instanceof Error ? error.message : String(error));
  } finally {
    workspacesLoading.set(false);
  }
}

export async function loadWorkspaceRoots(options: ReadRequestOptions = {}): Promise<WorkspaceRootView[]> {
  const roots = await listWorkspaceRoots(options);
  workspaceRoots.set(roots);
  return roots;
}

export async function browseWorkspaceRoot(rootId: string, path = '', options: ReadRequestOptions = {}): Promise<WorkspaceDirectoryListingView> {
  return listWorkspaceRootEntries(rootId, path, options);
}

export async function registerWorkspace(input: RegisterWorkspaceInput): Promise<WorkspaceView> {
  const workspace = await apiRegisterWorkspace(input);
  await loadWorkspaces();
  return workspace;
}

export async function renameWorkspace(workspaceId: string, input: RenameWorkspaceInput): Promise<WorkspaceView> {
  const workspace = await apiRenameWorkspace(workspaceId, input);
  await loadWorkspaces();
  return workspace;
}

export async function deleteWorkspace(workspaceId: string): Promise<WorkspaceView> {
  const workspace = await apiDeleteWorkspace(workspaceId);
  await loadWorkspaces();
  return workspace;
}
