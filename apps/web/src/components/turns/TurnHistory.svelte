<script lang="ts">
  import EmptyState from '../common/EmptyState.svelte';
  import ErrorBanner from '../common/ErrorBanner.svelte';
  import JsonView from '../common/JsonView.svelte';
  import LoadingState from '../common/LoadingState.svelte';
  import { turns, turnsError, turnsLoading } from '../../stores/turns';
</script>

<section class="panel">
  <h2>Turn history</h2>
  <ErrorBanner message={$turnsError} />
  {#if $turnsLoading}
    <LoadingState message="Loading turns..." />
  {:else if !$turns.length}
    <EmptyState message="No turns for this session." />
  {:else}
    <div class="timeline compact">
      {#each [...$turns].reverse() as turn (turn.turn_id)}
        <article class="timeline-item">
          <div class="panel-heading">
            <strong>{turn.turn_id}</strong>
            <span class="pill">{turn.state}</span>
          </div>
          <p class="muted">Created {turn.created_at}{turn.completed_at ? ` · Completed ${turn.completed_at}` : ''}</p>
          {#if turn.input?.summary}<p><strong>Input:</strong> {turn.input.summary}</p>{/if}
          {#if turn.output?.summary}<p><strong>Output:</strong> {turn.output.summary}</p>{/if}
          {#if turn.failure}<JsonView value={turn.failure} />{/if}
        </article>
      {/each}
    </div>
  {/if}
</section>
