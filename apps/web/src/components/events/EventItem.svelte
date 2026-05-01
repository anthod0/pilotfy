<script lang="ts">
  import JsonView from '../common/JsonView.svelte';
  import type { EventView } from '../../api/types';

  export let event: EventView;
</script>

<article class="timeline-item">
  <div class="panel-heading">
    <strong>{event.type}</strong>
    <span class="pill">{event.source}</span>
  </div>
  <p class="muted">{event.time} · {event.event_id}{event.turn_id ? ` · ${event.turn_id}` : ''}</p>
  <JsonView value={event.payload} />
</article>
