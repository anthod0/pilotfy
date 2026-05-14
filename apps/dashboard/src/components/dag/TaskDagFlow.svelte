<script lang="ts">
  import { Background, BackgroundVariant, Controls, MiniMap, SvelteFlow, type NodeTypes } from '@xyflow/svelte'
  import type { TaskDagView } from '../../api/types'
  import WorkItemNode from './WorkItemNode.svelte'
  import { buildDagFlow, type WorkItemFlowEdge, type WorkItemFlowNode } from './dagGraph'

  let { dag }: { dag: TaskDagView } = $props()

  const nodeTypes: NodeTypes = { workItem: WorkItemNode }

  let nodes = $state.raw<WorkItemFlowNode[]>([])
  let edges = $state.raw<WorkItemFlowEdge[]>([])
  $effect(() => {
    const flow = buildDagFlow(dag)
    nodes = flow.nodes
    edges = flow.edges
  })
</script>

<div class="h-[34rem] overflow-hidden rounded-lg border bg-background">
  <SvelteFlow
    bind:nodes
    bind:edges
    {nodeTypes}
    fitView
    nodesDraggable={false}
    nodesConnectable={false}
    elementsSelectable
    minZoom={0.2}
    maxZoom={1.5}
    proOptions={{ hideAttribution: true }}
  >
    <Background variant={BackgroundVariant.Dots} />
    <Controls />
    <MiniMap pannable zoomable />
  </SvelteFlow>
</div>
