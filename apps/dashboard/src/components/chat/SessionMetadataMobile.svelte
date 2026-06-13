<script lang="ts">
  import { GitBranch } from '@lucide/svelte'
  import type { SessionView, WorkspaceGitStatusView, WorkspaceView } from '../../api/types'
  import GitStatusInline from './GitStatusInline.svelte'
  import {
    gitStatusAriaLabel,
    gitStatusTitle,
    sessionContextUsageLabel,
    sessionHandleTitle,
    sessionProfileTitle,
    sessionWorkspaceTitle,
    type SessionMetadataItem,
  } from './sessionMetadata'

  interface Props {
    session: SessionView
    gitStatus?: WorkspaceGitStatusView
    gitStatusErrors?: Record<string, string | null | undefined>
    workspaces: WorkspaceView[]
    metadataItems: SessionMetadataItem[]
    metadataSummary: string
  }

  let { session, gitStatus, gitStatusErrors = {}, workspaces, metadataItems, metadataSummary }: Props = $props()
  let sessionDetailsOpen = $state(false)
</script>

<button type="button" class="flex h-7 w-full min-w-0 items-center justify-start bg-transparent px-0 text-sm text-muted-foreground outline-none hover:bg-transparent hover:text-foreground focus-visible:text-foreground" aria-haspopup="dialog" aria-expanded={sessionDetailsOpen} aria-label={`Session details: ${metadataSummary}`} onclick={() => (sessionDetailsOpen = !sessionDetailsOpen)}>
  <span data-chat-session-details-summary class="flex min-w-0 flex-1 items-center gap-2 overflow-hidden text-left">
    <span class="min-w-0 shrink truncate">{sessionWorkspaceTitle(session, workspaces)}</span>
    {#if gitStatus}
      <span
        class="inline-flex h-7 shrink-0 items-center gap-1.5 text-sm font-normal text-muted-foreground"
        title={gitStatusTitle(session, gitStatus, gitStatusErrors)}
        aria-label={gitStatusAriaLabel(gitStatus)}
      >
        <GitBranch class="size-4" aria-label="Git branch" />
        <GitStatusInline {gitStatus} />
      </span>
    {/if}
    {#if sessionContextUsageLabel(session)}
      <span class="shrink-0 text-muted-foreground">{sessionContextUsageLabel(session)}</span>
    {/if}
    <span class="shrink-0 text-muted-foreground">{session.client_type}</span>
    {#if sessionProfileTitle(session)}<span class="shrink-0 text-muted-foreground">{sessionProfileTitle(session)}</span>{/if}
    {#if sessionHandleTitle(session)}<span class="shrink-0 text-muted-foreground">{sessionHandleTitle(session)}</span>{/if}
  </span>
</button>
{#if sessionDetailsOpen}
  <div role="dialog" aria-label="Session details" class="absolute bottom-full left-0 z-20 mb-2 w-[min(20rem,calc(100vw-2rem))] rounded-lg border bg-popover p-3 text-popover-foreground shadow-md">
    <div class="mb-2 text-sm font-medium">Session details</div>
    <dl class="space-y-2 text-sm">
      {#each metadataItems as item (item.key)}
        <div class="grid grid-cols-[5.5rem_minmax(0,1fr)] gap-2">
          <dt class="text-muted-foreground">{item.label}</dt>
          <dd class="min-w-0 truncate" title={item.title}>{item.value}</dd>
        </div>
      {/each}
    </dl>
  </div>
{/if}
