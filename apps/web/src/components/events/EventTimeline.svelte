<script lang="ts">
  import EmptyState from '../common/EmptyState.svelte';
  import ErrorBanner from '../common/ErrorBanner.svelte';
  import LoadingState from '../common/LoadingState.svelte';
  import EventItem from './EventItem.svelte';
  import { events, eventsError, eventsLoading } from '../../stores/events';
</script>

<section class="panel">
  <div class="panel-heading">
    <h2>Event timeline</h2>
    <span class="muted">{$events.length} events</span>
  </div>
  <ErrorBanner message={$eventsError} />
  {#if $eventsLoading}
    <LoadingState message="Loading events..." />
  {:else if !$events.length}
    <EmptyState message="No events for this session." />
  {:else}
    <div class="timeline">
      {#each $events as event (event.event_id)}
        <EventItem {event} />
      {/each}
    </div>
  {/if}
</section>
