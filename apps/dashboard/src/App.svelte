<script lang="ts">
  import { onDestroy, onMount } from 'svelte'
  import { Router } from 'svelte-mini-router'
  import AppShell from './components/layout/AppShell.svelte'
  import { Toaster } from './lib/components/ui/sonner/index.js'
  import { startEventStream, stopEventStream } from './services/eventStream'
  import { loadAgentProfiles } from './stores/agentProfiles'
  import { token } from './stores/auth'
  import { loadSessions } from './stores/sessions'
  import { subscribeAfterInitial } from './stores/subscribeAfterInitial'
  import { loadTasks } from './stores/tasks'
  import { loadWorkspaces } from './stores/workspaces'
  import { routerConf } from './routes'

  let unsubscribeToken: (() => void) | null = null

  onMount(() => {
    void Promise.all([loadTasks(), loadWorkspaces(), loadAgentProfiles(), loadSessions()])
    startEventStream()
    unsubscribeToken = subscribeAfterInitial(token, () => {
      stopEventStream()
      startEventStream()
    })
  })

  onDestroy(() => {
    unsubscribeToken?.()
    stopEventStream()
  })
</script>

<Toaster richColors />

<AppShell>
  <Router {routerConf} />
</AppShell>
