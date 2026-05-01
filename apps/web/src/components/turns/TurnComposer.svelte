<script lang="ts">
  import { get } from 'svelte/store';
  import ErrorBanner from '../common/ErrorBanner.svelte';
  import { loadEvents } from '../../stores/events';
  import { selectedSessionId } from '../../stores/selection';
  import { refreshSession, session } from '../../stores/sessionDetail';
  import { submitTurn, turnsError } from '../../stores/turns';
  import { setStatus } from '../../stores/ui';

  let input = '';
  let submitting = false;

  $: busy = Boolean($session?.current_turn_id);
  $: disabled = submitting || busy || !input.trim() || !$selectedSessionId;

  async function submit() {
    const id = get(selectedSessionId);
    const text = input.trim();
    if (!id || !text || busy) return;
    submitting = true;
    try {
      await submitTurn(id, { input: text });
      await Promise.all([refreshSession(id), loadEvents(id)]);
      input = '';
      setStatus('Turn submitted.');
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    } finally {
      submitting = false;
    }
  }
</script>

<section class="panel">
  <div class="panel-heading">
    <h2>Submit turn</h2>
    {#if busy}<span class="pill">busy: {$session?.current_turn_id}</span>{/if}
  </div>
  <ErrorBanner message={$turnsError} />
  <label>
    Input
    <textarea bind:value={input} placeholder="Ask the selected session to do the next task" on:keydown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') void submit(); }}></textarea>
  </label>
  <button disabled={disabled} on:click={submit}>{submitting ? 'Submitting...' : busy ? 'Session busy' : 'Submit turn'}</button>
</section>
