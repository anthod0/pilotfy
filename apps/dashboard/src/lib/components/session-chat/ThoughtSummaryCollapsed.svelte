<script lang="ts">
  import { cn } from '$lib/utils.js'
  import { CheckCircle2, CircleDot, LoaderCircle, XCircle } from '@lucide/svelte'
  import type { SessionChatThoughtStep } from '../../session-chat/sessionChat'

  interface Props {
    step: SessionChatThoughtStep | null
    stepCount: number
    active?: boolean
    class?: string
    onOpen: () => void
  }

  let { step, stepCount, active = false, class: className, onOpen }: Props = $props()

  function labelForStep(thoughtStep: SessionChatThoughtStep | null): string {
    if (!thoughtStep) return 'Working'
    if (thoughtStep.kind === 'thinking') return 'Thinking'
    return thoughtStep.title || (thoughtStep.kind === 'tool_call' ? 'Tool call' : 'Tool result')
  }

  function statusForStep(thoughtStep: SessionChatThoughtStep | null): string | null {
    return thoughtStep?.status ?? (active ? 'working' : null)
  }

  const status = $derived(statusForStep(step))
</script>

<button
  type="button"
  class={cn(
    'not-prose w-full rounded-xl px-3 py-2.5 text-left transition focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring',
    className,
  )}
  aria-label="View thought details"
  onclick={onOpen}
>
  <div class="flex min-w-0 items-start">
    <span class="min-w-0 flex-1">
      <span class="flex min-w-0 items-center gap-2">
        <span class="truncate text-sm font-medium leading-5 text-foreground">{labelForStep(step)}</span>
        {#if status}
          <span class="inline-flex size-4 shrink-0 items-center justify-center text-muted-foreground" aria-label={status} title={status}>
            {#if status === 'completed'}
              <CheckCircle2 class="size-4" aria-hidden="true" />
            {:else if status === 'failed' || status === 'error'}
              <XCircle class="size-4 text-destructive" aria-hidden="true" />
            {:else if status === 'started' || status === 'working'}
              <LoaderCircle class="size-4 animate-spin" aria-hidden="true" />
            {:else}
              <CircleDot class="size-4" aria-hidden="true" />
            {/if}
          </span>
        {/if}
        {#if active && !status}<span class="size-2 shrink-0 animate-pulse rounded-full bg-primary" aria-label="Working"></span>{/if}
      </span>
      <span class="mt-1 line-clamp-1 whitespace-pre-wrap break-words text-xs leading-5 text-muted-foreground">
        {#if step?.content}{step.content}{:else}Working…{/if}
      </span>
    </span>
  </div>
</button>
