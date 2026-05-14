<script lang="ts">
  import { onMount } from 'svelte'
  import { CircleAlert, FolderOpen, RefreshCw, Trash2 } from '@lucide/svelte'
  import * as Alert from '$lib/components/ui/alert/index.js'
  import { Badge } from '$lib/components/ui/badge/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import * as Card from '$lib/components/ui/card/index.js'
  import * as Empty from '$lib/components/ui/empty/index.js'
  import { Input } from '$lib/components/ui/input/index.js'
  import { Label } from '$lib/components/ui/label/index.js'
  import { Skeleton } from '$lib/components/ui/skeleton/index.js'
  import * as Table from '$lib/components/ui/table/index.js'
  import { formatDateTime } from '../components/tasks/format'
  import type { WorkspaceDirectoryListingView } from '../api/types'
  import { browseWorkspaceRoot, deleteWorkspace, loadWorkspaceRoots, loadWorkspaces, registerWorkspace, workspaceRoots, workspaces, workspacesError, workspacesLoading } from '../stores/workspaces'

  let rootId = ''
  let browsePath = ''
  let listing: WorkspaceDirectoryListingView | null = null
  let browserLoading = false
  let browserError: string | null = null
  let workspaceName = ''
  let registering = false
  let registerError: string | null = null
  let deletingWorkspaceId: string | null = null
  let deleteError: string | null = null

  onMount(async () => {
    await Promise.all([loadWorkspaces(), loadWorkspaceRoots().then((roots) => {
      if (!rootId && roots.length) rootId = roots[0].root_id
    })])
    if (rootId) await openPath('')
  })

  $: selectedRoot = $workspaceRoots.find((root) => root.root_id === rootId) ?? null
  $: canRegister = rootId.trim().length > 0 && (listing?.canonical_path || browsePath.trim()).length > 0 && !registering

  async function refreshAll(): Promise<void> {
    await Promise.all([loadWorkspaces(), loadWorkspaceRoots()])
    if (rootId) await openPath(browsePath)
  }

  async function openPath(path: string): Promise<void> {
    if (!rootId) return
    browserLoading = true
    browserError = null
    try {
      listing = await browseWorkspaceRoot(rootId, path)
      browsePath = listing.path
    } catch (error) {
      listing = null
      browserError = error instanceof Error ? error.message : String(error)
    } finally {
      browserLoading = false
    }
  }

  async function registerCurrentWorkspace(): Promise<void> {
    if (!canRegister) return
    registering = true
    registerError = null
    try {
      await registerWorkspace({ root_id: rootId, path: listing?.path ?? browsePath, name: workspaceName.trim() || null })
      workspaceName = ''
    } catch (error) {
      registerError = error instanceof Error ? error.message : String(error)
    } finally {
      registering = false
    }
  }

  async function deleteRegisteredWorkspace(workspaceId: string, label: string): Promise<void> {
    if (deletingWorkspaceId || !confirm(`Delete workspace "${label}" from llmparty? Files on disk will not be deleted.`)) return
    deletingWorkspaceId = workspaceId
    deleteError = null
    try {
      await deleteWorkspace(workspaceId)
      if (rootId) await openPath(browsePath)
    } catch (error) {
      deleteError = error instanceof Error ? error.message : String(error)
    } finally {
      deletingWorkspaceId = null
    }
  }
</script>

<section class="space-y-6">
  <div class="flex flex-col gap-3 md:flex-row md:items-end md:justify-between">
    <div class="space-y-2">
      <Badge variant="secondary">Configuration</Badge>
      <h2 class="text-3xl font-semibold tracking-tight">Workspaces</h2>
      <p class="max-w-3xl text-muted-foreground">Browse configured roots and register execution workspaces through the External API.</p>
    </div>
    <Button variant="outline" onclick={() => void refreshAll()}><RefreshCw class="size-4" /> Refresh</Button>
  </div>

  {#if $workspacesError || browserError || registerError || deleteError}
    <Alert.Root variant="destructive">
      <CircleAlert class="size-4" />
      <Alert.Title>Workspace error</Alert.Title>
      <Alert.Description>{deleteError ?? registerError ?? browserError ?? $workspacesError}</Alert.Description>
    </Alert.Root>
  {/if}

  <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.8fr)]">
    <Card.Root>
      <Card.Header>
        <Card.Title class="flex items-center gap-2"><FolderOpen class="size-5" /> Root browser</Card.Title>
        <Card.Description>Select a root, browse directories, then register the current directory as a workspace.</Card.Description>
      </Card.Header>
      <Card.Content class="space-y-4">
        <div class="grid gap-3 md:grid-cols-[220px_1fr_auto] md:items-end">
          <div class="space-y-2">
            <Label for="workspace-root">Root</Label>
            <select id="workspace-root" bind:value={rootId} onchange={() => void openPath('')} class="h-9 w-full rounded-md border bg-transparent px-3 text-sm">
              {#each $workspaceRoots as root}
                <option value={root.root_id}>{root.label}</option>
              {/each}
            </select>
          </div>
          <div class="space-y-2">
            <Label for="browse-path">Path</Label>
            <Input id="browse-path" bind:value={browsePath} placeholder="Relative path inside root" />
          </div>
          <Button variant="outline" onclick={() => void openPath(browsePath)} disabled={!rootId || browserLoading}>Open</Button>
        </div>

        {#if selectedRoot}
          <p class="text-xs text-muted-foreground">Root state: {selectedRoot.state} · {selectedRoot.canonical_path ?? 'virtual root'}</p>
        {/if}

        {#if browserLoading}
          <div class="space-y-2"><Skeleton class="h-9 w-full" /><Skeleton class="h-9 w-full" /><Skeleton class="h-9 w-full" /></div>
        {:else if listing}
          <div class="rounded-lg border">
            <div class="flex flex-wrap items-center justify-between gap-2 border-b p-3 text-sm">
              <span class="font-medium">{listing.canonical_path}</span>
              {#if listing.parent_path !== null}<Button size="sm" variant="ghost" onclick={() => void openPath(listing?.parent_path ?? '')}>Up</Button>{/if}
            </div>
            {#if listing.warnings.length}
              <div class="border-b bg-muted/40 p-3 text-xs text-muted-foreground">{listing.warnings.join(' · ')}</div>
            {/if}
            <div class="max-h-[28rem] overflow-auto">
              <Table.Root>
                <Table.Header><Table.Row><Table.Head>Name</Table.Head><Table.Head>Kind</Table.Head><Table.Head>Workspace</Table.Head><Table.Head class="text-right">Action</Table.Head></Table.Row></Table.Header>
                <Table.Body>
                  {#each listing.entries as entry}
                    <Table.Row>
                      <Table.Cell class="font-medium">{entry.name}</Table.Cell>
                      <Table.Cell>{entry.kind}</Table.Cell>
                      <Table.Cell>{entry.is_workspace ? 'registered' : '—'}</Table.Cell>
                      <Table.Cell class="text-right">
                        <Button size="sm" variant="outline" disabled={entry.kind !== 'directory'} onclick={() => void openPath(entry.path)}>Open</Button>
                      </Table.Cell>
                    </Table.Row>
                  {/each}
                </Table.Body>
              </Table.Root>
            </div>
          </div>
        {:else}
          <Empty.Root><Empty.Header><Empty.Title>No root opened</Empty.Title><Empty.Description>Select a workspace root to browse.</Empty.Description></Empty.Header></Empty.Root>
        {/if}
      </Card.Content>
    </Card.Root>

    <div class="space-y-4">
      <Card.Root>
        <Card.Header><Card.Title>Register current directory</Card.Title><Card.Description>Registers the open browser path as an execution workspace.</Card.Description></Card.Header>
        <Card.Content class="space-y-3">
          <div class="space-y-2">
            <Label for="workspace-name">Display name (optional)</Label>
            <Input id="workspace-name" bind:value={workspaceName} placeholder="llmparty" />
          </div>
          <p class="text-xs text-muted-foreground">Path: {listing?.canonical_path ?? (browsePath || '—')}</p>
          <Button onclick={registerCurrentWorkspace} disabled={!canRegister}>{registering ? 'Registering…' : 'Register workspace'}</Button>
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header><Card.Title>Registered workspaces</Card.Title><Card.Description>{$workspaces.length} available for DAG task creation.</Card.Description></Card.Header>
        <Card.Content>
          {#if $workspacesLoading}
            <div class="space-y-2"><Skeleton class="h-10 w-full" /><Skeleton class="h-10 w-full" /></div>
          {:else if !$workspaces.length}
            <Empty.Root><Empty.Header><Empty.Title>No workspaces</Empty.Title><Empty.Description>Register one from a root above.</Empty.Description></Empty.Header></Empty.Root>
          {:else}
            <div class="space-y-3">
              {#each $workspaces as workspace}
                {@const workspaceLabel = workspace.name ?? workspace.display_path}
                <div class="rounded-lg border p-3 text-sm">
                  <div class="flex items-start justify-between gap-3">
                    <div class="min-w-0">
                      <div class="font-medium">{workspaceLabel}</div>
                      <div class="truncate text-muted-foreground" title={workspace.canonical_path}>{workspace.canonical_path}</div>
                    </div>
                    <Button
                      size="sm"
                      variant="outline"
                      onclick={() => void deleteRegisteredWorkspace(workspace.workspace_id, workspaceLabel)}
                      disabled={deletingWorkspaceId === workspace.workspace_id}
                    >
                      <Trash2 class="size-4" />
                      {deletingWorkspaceId === workspace.workspace_id ? 'Deleting…' : 'Delete'}
                    </Button>
                  </div>
                  <div class="mt-2 flex flex-wrap gap-2 text-xs text-muted-foreground"><Badge variant="secondary">{workspace.state}</Badge><span>Updated {formatDateTime(workspace.updated_at)}</span></div>
                </div>
              {/each}
            </div>
          {/if}
        </Card.Content>
      </Card.Root>
    </div>
  </div>
</section>
