<script lang="ts">
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Card from '$lib/components/ui/card/index.js';
</script>

<main class="flex min-h-screen items-center justify-center bg-background p-6 text-foreground">
  <Card.Root class="w-full max-w-xl">
    <Card.Header>
      <Card.Title>llmparty Dashboard v2</Card.Title>
      <Card.Description>
        New dashboard scaffold initialized with Svelte, Vite, Tailwind CSS, shadcn-svelte, and SPA routing dependencies.
      </Card.Description>
    </Card.Header>
    <Card.Content class="space-y-2 text-sm text-muted-foreground">
      <p>Build this app and configure <code>[dashboard].source = "apps/dashboard/dist"</code> to serve it at <code>/dashboard</code>.</p>
      <p>The full implementation should follow <code>specs/plans/2026-05-13-dashboard-v2.md</code>.</p>
    </Card.Content>
    <Card.Footer>
      <Button href="/dashboard/tasks">Open Tasks</Button>
    </Card.Footer>
  </Card.Root>
</main>
