<script lang="ts">
  import { derived } from 'svelte/store';
  import EmptyState from '../common/EmptyState.svelte';
  import { latestEventOutput } from '../../stores/events';
  import { latestOutput } from '../../stores/turns';

  const reply = derived([latestEventOutput, latestOutput], ([$eventOutput, $turnOutput]) => $eventOutput ?? $turnOutput);
</script>

<section class="panel">
  <h2>Latest reply</h2>
  {#if $reply && $reply !== 'No turn output yet.'}
    <pre>{$reply}</pre>
  {:else}
    <EmptyState message="No turn output yet." />
  {/if}
</section>
