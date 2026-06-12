<script lang="ts">
  import ThoughtSummaryCollapsed from './ThoughtSummaryCollapsed.svelte'
  import ThoughtSummaryIdle from './ThoughtSummaryIdle.svelte'
  import ThoughtSummarySheet from './ThoughtSummarySheet.svelte'
  import type { SessionChatThoughtStep } from '../../session-chat/sessionChat'

  interface Props {
    steps: SessionChatThoughtStep[]
    active?: boolean
    class?: string
  }

  let { steps, active = false, class: className }: Props = $props()
  let sheetOpen = $state(false)
</script>

{#if active}
  <ThoughtSummaryCollapsed {steps} {active} class={className} onOpen={() => (sheetOpen = true)} />
{:else}
  <ThoughtSummaryIdle count={steps.length} class={className} onOpen={() => (sheetOpen = true)} />
{/if}
<ThoughtSummarySheet bind:open={sheetOpen} {steps} {active} />
