<script lang="ts">
  import { tick } from 'svelte'
  import { Bot, UserRound } from '@lucide/svelte'
  import * as Conversation from '$lib/components/ai-elements/conversation/index.js'
  import * as Message from '$lib/components/ai-elements/message/index.js'
  import * as Empty from '$lib/components/ui/empty/index.js'
  import { Badge } from '$lib/components/ui/badge/index.js'
  import { chatAutoScrollKey, scrollToBottom } from '../../session-chat/autoScroll'
  import type { SessionChatMessage } from '../../session-chat/sessionChat'

  interface Props {
    messages: SessionChatMessage[]
    loading?: boolean
  }

  let { messages, loading = false }: Props = $props()
  let scrollContainer = $state<HTMLDivElement | null>(null)
  const scrollKey = $derived(chatAutoScrollKey(messages))

  $effect(() => {
    scrollKey
    void tick().then(() => scrollToBottom(scrollContainer))
  })
</script>

<Conversation.Root class="min-h-0 flex-1">
  {#if loading}
    <Conversation.EmptyState title="Loading conversation…" description="Fetching the latest session turns." />
  {:else if !messages.length}
    <Empty.Root class="h-full">
      <Empty.Header>
        <Empty.Media><Bot class="size-6" /></Empty.Media>
        <Empty.Title>No messages yet</Empty.Title>
        <Empty.Description>This session has no turn history yet.</Empty.Description>
      </Empty.Header>
    </Empty.Root>
  {:else}
    <Conversation.Content bind:ref={scrollContainer}>
      {#each messages as chatMessage (chatMessage.id)}
        <Message.Root from={chatMessage.role}>
          <div class="mb-1 flex items-center gap-2 text-xs text-muted-foreground {chatMessage.role === 'user' ? 'justify-end' : 'justify-start'}">
            {#if chatMessage.role === 'assistant'}<Bot class="size-3.5" />{:else}<UserRound class="size-3.5" />{/if}
            <span>{chatMessage.role === 'assistant' ? 'AI' : 'You'}</span>
            {#if chatMessage.status !== 'sent'}<Badge variant="secondary">{chatMessage.status}</Badge>{/if}
          </div>
          <Message.Content class={chatMessage.status === 'failed' ? 'border-destructive/40 text-destructive' : ''}>
            <Message.Response content={chatMessage.content} />
          </Message.Content>
        </Message.Root>
      {/each}
    </Conversation.Content>
  {/if}
</Conversation.Root>
