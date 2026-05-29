<script lang="ts">
  import { Toaster as Sonner, type ToasterProps } from 'svelte-sonner'

  let { ...restProps }: ToasterProps = $props()
</script>

<Sonner
  class="toaster group"
  style="--normal-bg: var(--color-popover); --normal-text: var(--color-popover-foreground); --normal-border: var(--color-border);"
  {...restProps}
/>
