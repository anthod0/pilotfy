<script lang="ts">
  import { onMount } from 'svelte';
  import { createSession } from '../../api/client';
  import { loadSessions } from '../../stores/sessions';
  import { selectSession } from '../../stores/selection';
  import { setStatus } from '../../stores/ui';
  import {
    browseWorkspaceRoot,
    loadWorkspaceRoots,
    loadWorkspaces,
    registerWorkspace,
    workspaceRoots,
    workspaces,
  } from '../../stores/workspaces';
  import type { WorkspaceDirectoryListingView } from '../../api/types';

  let clientType = 'claude_code';
  let workspaceId = '';
  let handle = '';
  let initialTask = '';
  let creating = false;
  let registering = false;
  let browserOpen = false;
  let selectedRootId = '';
  let listing: WorkspaceDirectoryListingView | null = null;
  let selectedPath = '';
  let workspaceName = '';

  onMount(async () => {
    await loadWorkspaces();
  });

  async function submit() {
    creating = true;
    try {
      const result = await createSession({
        client_type: clientType,
        workspace_id: workspaceId || null,
        handle: handle.trim() || null,
        initial_task: initialTask.trim() ? { input: initialTask.trim() } : null,
      });
      await loadSessions();
      await selectSession(result.session.session_id);
      handle = '';
      initialTask = '';
      setStatus('Session created.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    } finally {
      creating = false;
    }
  }

  async function openBrowser() {
    browserOpen = true;
    try {
      const roots = await loadWorkspaceRoots();
      selectedRootId = selectedRootId || roots.find((root) => root.state === 'available')?.root_id || roots[0]?.root_id || '';
      if (selectedRootId) await browse('');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    }
  }

  async function browse(path: string) {
    if (!selectedRootId) return;
    try {
      listing = await browseWorkspaceRoot(selectedRootId, path);
      selectedPath = listing.path;
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    }
  }

  async function registerSelectedWorkspace() {
    if (!selectedRootId) return;
    registering = true;
    try {
      const workspace = await registerWorkspace({
        root_id: selectedRootId,
        path: selectedPath,
        name: workspaceName.trim() || null,
      });
      workspaceId = workspace.workspace_id;
      browserOpen = false;
      workspaceName = '';
      setStatus('Workspace registered.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    } finally {
      registering = false;
    }
  }
</script>

<section class="panel">
  <h2>Create session</h2>
  <label>Client type <select bind:value={clientType}><option value="claude_code">claude_code</option><option value="pi">pi</option></select></label>
  <label>
    Workspace
    <select bind:value={workspaceId}>
      <option value="">Select a known workspace</option>
      {#each $workspaces as workspace (workspace.workspace_id)}
        <option value={workspace.workspace_id}>{workspace.name ?? workspace.display_path}</option>
      {/each}
    </select>
  </label>
  <button class="secondary" type="button" on:click={openBrowser}>Register workspace</button>

  {#if browserOpen}
    <div class="nested-panel">
      <label>
        Root
        <select bind:value={selectedRootId} on:change={() => browse('')}>
          {#each $workspaceRoots as root (root.root_id)}
            <option value={root.root_id} disabled={root.state !== 'available'}>{root.label} · {root.state}</option>
          {/each}
        </select>
      </label>
      {#if listing}
        <small class="muted">{listing.canonical_path}</small>
        <div class="row">
          <button class="secondary" type="button" disabled={listing.parent_path === null} on:click={() => browse(listing?.parent_path ?? '')}>Up</button>
          <button class="secondary" type="button" on:click={() => { selectedPath = listing?.path ?? ''; }}>Use this directory</button>
        </div>
        <div class="list compact">
          {#each listing.entries as entry (entry.path)}
            <button class="item" type="button" on:click={() => browse(entry.path)}>
              <strong>{entry.name}</strong>
              <span>{entry.is_workspace ? 'known workspace' : entry.path}</span>
            </button>
          {/each}
        </div>
        <label>Display name <input bind:value={workspaceName} placeholder="Optional" /></label>
        <button disabled={registering} type="button" on:click={registerSelectedWorkspace}>{registering ? 'Registering...' : 'Register selected directory'}</button>
      {/if}
    </div>
  {/if}

  <label>Handle <input bind:value={handle} placeholder="@reviewer" /></label>
  <label>Initial task <textarea bind:value={initialTask} placeholder="Optional initial task"></textarea></label>
  <button disabled={creating || !workspaceId} on:click={submit}>{creating ? 'Creating...' : 'Create session'}</button>
</section>
