<script lang="ts">
  import { onMount } from 'svelte';
  import { createDagTask, createTask } from '../../stores/tasks';
  import { loadSessions } from '../../stores/sessions';
  import { selectSession } from '../../stores/selection';
  import { loadWorkspaces } from '../../stores/workspaces';
  import { setStatus } from '../../stores/ui';
  import {
    agentProfiles,
    clientTypeOptionsForProfile,
    loadAgentProfiles,
    selectClientTypeForProfile,
  } from '../../stores/agentProfiles';
  import WorkspaceSelector from '../workspaces/WorkspaceSelector.svelte';
  import type { WorkspaceView } from '../../api/types';

  let taskMode = 'normal' as 'normal' | 'dag';
  let profileId = '';
  let clientType = 'pi';
  let workspaceId = '';
  let workspacePath = '';
  let input = '';
  let creating = false;

  $: selectedProfile = $agentProfiles.find((profile) => profile.profile_id === profileId) ?? null;
  $: clientTypeOptions = clientTypeOptionsForProfile(selectedProfile);

  onMount(() => {
    void loadAgentProfiles();
  });

  function applyProfileDefaults() {
    if (taskMode === 'dag' && !clientType) {
      clientType = 'pi';
      return;
    }
    clientType = selectClientTypeForProfile(clientType, selectedProfile);
  }

  function applyTaskMode() {
    if (taskMode === 'dag') clientType = 'pi';
    else clientType = selectClientTypeForProfile(clientType, selectedProfile);
  }

  function handleWorkspaceSelected(event: CustomEvent<WorkspaceView | null>) {
    workspacePath = event.detail?.canonical_path ?? '';
  }

  async function submit() {
    const taskInput = input.trim();
    if (!taskInput || (taskMode === 'dag' && !workspacePath)) return;
    creating = true;
    try {
      const payload = {
        input: taskInput,
        client_type: clientType,
        workspace: workspacePath || null,
        metadata: {},
      };
      if (taskMode === 'dag') {
        const result = await createDagTask(payload);
        await Promise.all([loadSessions(), loadWorkspaces()]);
        await selectSession(result.planning_turn.session_id);
        input = '';
        setStatus('DAG task created; planner turn started.');
        return;
      }
      const task = await createTask(payload);
      await Promise.all([loadSessions(), loadWorkspaces()]);
      if (task.session_id) await selectSession(task.session_id);
      input = '';
      setStatus(task.session_id ? 'Task created and dispatched.' : 'Task created; workspace confirmation may be required.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    } finally {
      creating = false;
    }
  }
</script>

<section class="panel">
  <h2>Create task</h2>
  <p class="muted">Use Normal for legacy workspace routing, or DAG task to start a planner-managed WorkItem DAG.</p>
  <fieldset>
    <legend>Task mode</legend>
    <label><input type="radio" bind:group={taskMode} value="normal" on:change={applyTaskMode} /> Normal task</label>
    <label><input type="radio" bind:group={taskMode} value="dag" on:change={applyTaskMode} /> DAG task</label>
  </fieldset>
  <label>Agent profile
    <select bind:value={profileId} on:change={applyProfileDefaults}>
      <option value="">No profile defaults</option>
      {#each $agentProfiles as profile (profile.profile_id)}
        <option value={profile.profile_id}>{profile.name} ({profile.profile_id}@{profile.version})</option>
      {/each}
    </select>
  </label>
  <label>Client type
    <select bind:value={clientType}>
      {#each clientTypeOptions as option (option)}
        <option value={option}>{option}</option>
      {/each}
    </select>
  </label>
  <WorkspaceSelector bind:selectedWorkspaceId={workspaceId} on:selected={handleWorkspaceSelected} />
  {#if taskMode === 'dag' && !workspacePath}<p class="muted">Workspace is required for DAG task planning.</p>{/if}
  <label>Task <textarea bind:value={input} placeholder="Ask the agent control layer to do work"></textarea></label>
  <button disabled={creating || !input.trim() || (taskMode === 'dag' && !workspacePath)} on:click={submit}>{creating ? 'Creating...' : 'Create task'}</button>
</section>
