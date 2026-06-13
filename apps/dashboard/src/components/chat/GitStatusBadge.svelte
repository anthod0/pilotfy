<script lang="ts">
  import { GitBranch } from '@lucide/svelte'
  import { Badge } from '$lib/components/ui/badge/index.js'
  import type { SessionView, WorkspaceGitStatusView } from '../../api/types'
  import { gitBranchLabel, gitStatusAriaLabel, gitStatusTitle, gitStatusToneClass, hasGitChangeCounts } from './sessionMetadata'

  interface Props {
    session: SessionView
    gitStatus: WorkspaceGitStatusView
    gitStatusErrors?: Record<string, string | null | undefined>
  }

  let { session, gitStatus, gitStatusErrors = {} }: Props = $props()
</script>

<Badge
  variant={gitStatus.state === 'error' ? 'destructive' : 'outline'}
  class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground"
  title={gitStatusTitle(session, gitStatus, gitStatusErrors)}
  aria-label={gitStatusAriaLabel(gitStatus)}
>
  <GitBranch class={`size-4 ${gitStatusToneClass(gitStatus)}`} aria-label="Git branch" />
  <span class={gitStatusToneClass(gitStatus)}>{gitBranchLabel(gitStatus)}</span>
  {#if gitStatus.ahead}<span class="text-blue-600 dark:text-blue-400">↑{gitStatus.ahead}</span>{/if}
  {#if gitStatus.behind}<span class="text-violet-600 dark:text-violet-400">↓{gitStatus.behind}</span>{/if}
  {#if hasGitChangeCounts(gitStatus)}
    {#if gitStatus.staged_count}<span class="text-emerald-600 dark:text-emerald-400">+{gitStatus.staged_count}</span>{/if}
    {#if gitStatus.unstaged_count}<span class="text-amber-600 dark:text-amber-400">~{gitStatus.unstaged_count}</span>{/if}
    {#if gitStatus.untracked_count}<span class="text-cyan-600 dark:text-cyan-400">?{gitStatus.untracked_count}</span>{/if}
    {#if gitStatus.conflicted_count}<span class="text-destructive">!{gitStatus.conflicted_count}</span>{/if}
  {/if}
</Badge>
