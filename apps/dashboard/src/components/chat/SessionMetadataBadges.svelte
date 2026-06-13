<script lang="ts">
  import { AtSign, Bot, Folder, Gauge, GitBranch, Terminal } from '@lucide/svelte'
  import { Badge } from '$lib/components/ui/badge/index.js'
  import type { SessionView, WorkspaceGitStatusView, WorkspaceView } from '../../api/types'
  import GitStatusInline from './GitStatusInline.svelte'
  import { gitStatusAriaLabel, gitStatusTitle, sessionContextUsageLabel, sessionHandleTitle, sessionProfileTitle, sessionWorkspacePath, sessionWorkspaceTitle } from './sessionMetadata'

  interface Props {
    session: SessionView
    gitStatus?: WorkspaceGitStatusView
    gitStatusErrors?: Record<string, string | null | undefined>
    workspaces: WorkspaceView[]
  }

  let { session, gitStatus, gitStatusErrors = {}, workspaces }: Props = $props()
</script>

<Badge variant="outline" class="h-7 max-w-full justify-start gap-1.5 px-3 text-sm font-normal text-muted-foreground" title={`Workspace: ${sessionWorkspacePath(session, workspaces)}`} aria-label={`Workspace: ${sessionWorkspacePath(session, workspaces)}`}>
  <Folder class="size-4" aria-hidden="true" />
  <span class="min-w-0 truncate">{sessionWorkspaceTitle(session, workspaces)}</span>
</Badge>
{#if gitStatus}
  <Badge
    variant={gitStatus.state === 'error' ? 'destructive' : 'outline'}
    class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground"
    title={gitStatusTitle(session, gitStatus, gitStatusErrors)}
    aria-label={gitStatusAriaLabel(gitStatus)}
  >
    <GitBranch class="size-4" aria-label="Git branch" />
    <GitStatusInline {gitStatus} />
  </Badge>
{/if}
{#if sessionContextUsageLabel(session)}
  <Badge variant="outline" class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground" title={`Context usage: ${sessionContextUsageLabel(session)}`} aria-label={`Context usage: ${sessionContextUsageLabel(session)}`}>
    <Gauge class="size-4" aria-hidden="true" /> {sessionContextUsageLabel(session)}
  </Badge>
{/if}
<Badge variant="outline" class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground" title={`Client: ${session.client_type}`} aria-label={`Client: ${session.client_type}`}>
  <Terminal class="size-4" aria-hidden="true" /> {session.client_type}
</Badge>
{#if sessionProfileTitle(session)}
  <Badge variant="outline" class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground" title={`Profile: ${sessionProfileTitle(session)}`} aria-label={`Profile: ${sessionProfileTitle(session)}`}>
    <Bot class="size-4" aria-hidden="true" /> {sessionProfileTitle(session)}
  </Badge>
{/if}
{#if sessionHandleTitle(session)}
  <Badge variant="outline" class="h-7 gap-1.5 px-3 text-sm font-normal text-muted-foreground" title={`Handle: ${sessionHandleTitle(session)}`} aria-label={`Handle: ${sessionHandleTitle(session)}`}>
    <AtSign class="size-4" aria-hidden="true" /> {sessionHandleTitle(session)}
  </Badge>
{/if}
