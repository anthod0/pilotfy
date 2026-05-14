<script lang="ts">
  import { CircleAlert, Eye, RefreshCw, Search } from '@lucide/svelte'
  import * as Alert from '$lib/components/ui/alert/index.js'
  import { Badge } from '$lib/components/ui/badge/index.js'
  import { Button } from '$lib/components/ui/button/index.js'
  import * as Card from '$lib/components/ui/card/index.js'
  import * as Empty from '$lib/components/ui/empty/index.js'
  import { Skeleton } from '$lib/components/ui/skeleton/index.js'
  import * as Table from '$lib/components/ui/table/index.js'
  import { formatDateTime, shortId } from '../../components/tasks/format'
  import { task, taskDag } from '../../stores/tasks'
  import { artifactContentError, artifactContentLoading, discoverTaskArtifacts, loadArtifactContent, loadTaskArtifacts, selectedArtifactContent, taskArtifacts, taskArtifactsError, taskArtifactsLoading } from '../../stores/artifacts'
  import TaskPageFrame from './TaskPageFrame.svelte'

  $: if ($task || $taskDag) void loadTaskArtifacts($task, $taskDag)

  function formatBytes(size: number | null): string {
    if (size === null) return '—'
    if (size < 1024) return `${size} B`
    if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KiB`
    return `${(size / 1024 / 1024).toFixed(1)} MiB`
  }

  function canPreview(contentType: string): boolean {
    return contentType.startsWith('text/') || contentType.includes('json') || contentType.includes('xml') || contentType.includes('javascript')
  }
</script>

<TaskPageFrame title="Artifacts" description="Task artifact discovery, list, and content viewer using External API artifact endpoints only.">
  <div class="space-y-4">
    <div class="flex flex-wrap justify-end gap-2">
      <Button variant="outline" onclick={() => void loadTaskArtifacts($task, $taskDag)}><RefreshCw class="size-4" /> Refresh</Button>
      <Button onclick={() => void discoverTaskArtifacts($task, $taskDag)}><Search class="size-4" /> Discover artifacts</Button>
    </div>

    {#if $taskArtifactsError || $artifactContentError}
      <Alert.Root variant="destructive">
        <CircleAlert class="size-4" />
        <Alert.Title>Artifact error</Alert.Title>
        <Alert.Description>{$artifactContentError ?? $taskArtifactsError}</Alert.Description>
      </Alert.Root>
    {/if}

    <div class="grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(22rem,0.85fr)]">
      <Card.Root>
        <Card.Header><Card.Title>Artifacts</Card.Title><Card.Description>{$taskArtifacts.length} artifacts from associated sessions.</Card.Description></Card.Header>
        <Card.Content>
          {#if $taskArtifactsLoading}
            <div class="space-y-2"><Skeleton class="h-10 w-full" /><Skeleton class="h-10 w-full" /><Skeleton class="h-10 w-full" /></div>
          {:else if !$taskArtifacts.length}
            <Empty.Root><Empty.Header><Empty.Title>No artifacts</Empty.Title><Empty.Description>No artifacts are recorded yet. Try discovery after task execution has produced files.</Empty.Description></Empty.Header></Empty.Root>
          {:else}
            <div class="overflow-x-auto">
              <Table.Root>
                <Table.Header><Table.Row><Table.Head>Name</Table.Head><Table.Head>Kind</Table.Head><Table.Head>Size</Table.Head><Table.Head>Session</Table.Head><Table.Head>Created</Table.Head><Table.Head class="text-right">View</Table.Head></Table.Row></Table.Header>
                <Table.Body>
                  {#each $taskArtifacts as artifact}
                    <Table.Row>
                      <Table.Cell><div class="font-medium">{artifact.name}</div><div class="max-w-xs truncate text-xs text-muted-foreground">{artifact.artifact_id}</div></Table.Cell>
                      <Table.Cell><Badge variant="secondary">{artifact.kind}</Badge></Table.Cell>
                      <Table.Cell>{formatBytes(artifact.size_bytes)}</Table.Cell>
                      <Table.Cell>{shortId(artifact.source_session_id)}</Table.Cell>
                      <Table.Cell>{formatDateTime(artifact.created_at)}</Table.Cell>
                      <Table.Cell class="text-right"><Button size="sm" variant="outline" onclick={() => void loadArtifactContent(artifact.artifact_id)}><Eye class="size-4" /> Open</Button></Table.Cell>
                    </Table.Row>
                  {/each}
                </Table.Body>
              </Table.Root>
            </div>
          {/if}
        </Card.Content>
      </Card.Root>

      <Card.Root>
        <Card.Header><Card.Title>Content viewer</Card.Title><Card.Description>Fetches /external/v1/artifacts/&lt;id&gt;/content with the saved bearer token.</Card.Description></Card.Header>
        <Card.Content>
          {#if $artifactContentLoading}
            <div class="space-y-2"><Skeleton class="h-8 w-2/3" /><Skeleton class="h-72 w-full" /></div>
          {:else if $selectedArtifactContent}
            <div class="space-y-3">
              <div class="flex flex-wrap items-center gap-2 text-sm"><Badge variant="secondary">{$selectedArtifactContent.contentType}</Badge><span class="text-muted-foreground">{formatBytes($selectedArtifactContent.bytes.byteLength)}</span></div>
              {#if canPreview($selectedArtifactContent.contentType)}
                <pre class="max-h-[32rem] overflow-auto whitespace-pre-wrap rounded-md bg-muted p-3 text-xs">{$selectedArtifactContent.text}</pre>
              {:else}
                <Empty.Root><Empty.Header><Empty.Title>Binary content</Empty.Title><Empty.Description>This artifact is not a text type. Download support can be added later if needed.</Empty.Description></Empty.Header></Empty.Root>
              {/if}
            </div>
          {:else}
            <Empty.Root><Empty.Header><Empty.Title>No artifact selected</Empty.Title><Empty.Description>Open an artifact from the list to inspect its content.</Empty.Description></Empty.Header></Empty.Root>
          {/if}
        </Card.Content>
      </Card.Root>
    </div>
  </div>
</TaskPageFrame>
