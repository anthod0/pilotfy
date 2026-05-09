<script lang="ts">
  import { get } from 'svelte/store';
  import ErrorBanner from '../common/ErrorBanner.svelte';
  import { loadEvents } from '../../stores/events';
  import { inboxError, lastSubmittedInboxMessage, submitInboxMessage } from '../../stores/inbox';
  import { selectedSessionId } from '../../stores/selection';
  import { refreshSession, session } from '../../stores/sessionDetail';
  import { loadTurns } from '../../stores/turns';
  import { setStatus } from '../../stores/ui';
  import type { InboxDeliveryPolicy } from '../../api/types';

  let input = '';
  let submitting: InboxDeliveryPolicy | null = null;

  $: busy = Boolean($session?.current_turn_id);
  $: canInterrupt = Boolean($session?.capabilities?.interrupt);
  $: disabled = Boolean(submitting) || !input.trim() || !$selectedSessionId;

  async function submit(deliveryPolicy: InboxDeliveryPolicy = 'after_idle') {
    const id = get(selectedSessionId);
    const text = input.trim();
    if (!id || !text) return;
    submitting = deliveryPolicy;
    try {
      const message = await submitInboxMessage(id, {
        input: text,
        delivery_policy: deliveryPolicy,
        metadata: { source: 'dashboard' },
      });
      await Promise.all([refreshSession(id), loadTurns(id), loadEvents(id)]);
      input = '';
      setStatus(message.state === 'dispatched' ? 'Message dispatched.' : `Message ${message.state}.`);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : String(error), true);
    } finally {
      submitting = null;
    }
  }
</script>

<section class="panel">
  <div class="panel-heading">
    <h2>Session inbox</h2>
    {#if busy}<span class="pill">busy: {$session?.current_turn_id}</span>{/if}
  </div>
  <ErrorBanner message={$inboxError} />
  <label>
    Input
    <textarea bind:value={input} placeholder="Ask the selected session to do the next task" on:keydown={(event) => { if ((event.metaKey || event.ctrlKey) && event.key === 'Enter') void submit('after_idle'); }}></textarea>
  </label>
  <div class="composer-actions">
    <button disabled={disabled} on:click={() => submit('after_idle')}>
      {submitting === 'after_idle' ? 'Queueing...' : busy ? 'Queue after current turn' : 'Send'}
    </button>
    {#if busy}
      <button disabled={disabled || !canInterrupt} on:click={() => submit('interrupt_now')} title={canInterrupt ? 'Interrupt current turn and send next' : 'Runtime does not advertise interrupt support'}>
        {submitting === 'interrupt_now' ? 'Interrupting...' : 'Interrupt and send next'}
      </button>
    {/if}
  </div>
  {#if $lastSubmittedInboxMessage}
    <p class="muted">
      Last inbox message: {$lastSubmittedInboxMessage.state}
      {#if $lastSubmittedInboxMessage.turn_id} → {$lastSubmittedInboxMessage.turn_id}{/if}
      {#if $lastSubmittedInboxMessage.failure_message} — {$lastSubmittedInboxMessage.failure_message}{/if}
    </p>
  {/if}
</section>

<style>
  .composer-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
  }

  .muted {
    color: var(--muted, #667085);
    font-size: 0.9rem;
  }
</style>
