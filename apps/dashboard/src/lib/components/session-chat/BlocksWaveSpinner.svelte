<script lang="ts">
  import { cn } from '$lib/utils.js'

  interface Props {
    class?: string
  }

  let { class: className }: Props = $props()
</script>

<svg
  data-testid="blocks-wave-spinner"
  class={cn('blocks-wave-spinner', className)}
  width="24"
  height="24"
  viewBox="0 0 24 24"
  fill="currentColor"
  xmlns="http://www.w3.org/2000/svg"
  aria-hidden="true"
>
  <rect class="spinner-zWVm" x="1" y="1" width="7.33" height="7.33" />
  <rect class="spinner-gfyD" x="8.33" y="1" width="7.33" height="7.33" />
  <rect class="spinner-T5JJ" x="1" y="8.33" width="7.33" height="7.33" />
  <rect class="spinner-E3Wz" x="15.66" y="1" width="7.33" height="7.33" />
  <rect class="spinner-g2vs" x="8.33" y="8.33" width="7.33" height="7.33" />
  <rect class="spinner-ctYB" x="1" y="15.66" width="7.33" height="7.33" />
  <rect class="spinner-BDNj" x="15.66" y="8.33" width="7.33" height="7.33" />
  <rect class="spinner-rCw3" x="8.33" y="15.66" width="7.33" height="7.33" />
  <rect class="spinner-Rszm" x="15.66" y="15.66" width="7.33" height="7.33" />
</svg>

<style>
  /* Adapted from https://github.com/n3r4zzurr0/svg-spinners/blob/main/svg-css/blocks-wave.svg */
  .spinner-zWVm {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-1 1.2s linear infinite;
  }

  .spinner-gfyD {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-2 1.2s linear infinite;
    animation-delay: 0.1s;
  }

  .spinner-T5JJ {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-3 1.2s linear infinite;
    animation-delay: 0.1s;
  }

  .spinner-E3Wz {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-4 1.2s linear infinite;
    animation-delay: 0.2s;
  }

  .spinner-g2vs {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-5 1.2s linear infinite;
    animation-delay: 0.2s;
  }

  .spinner-ctYB {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-6 1.2s linear infinite;
    animation-delay: 0.2s;
  }

  .spinner-BDNj {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-7 1.2s linear infinite;
    animation-delay: 0.3s;
  }

  .spinner-rCw3 {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-8 1.2s linear infinite;
    animation-delay: 0.3s;
  }

  .spinner-Rszm {
    animation: blocks-wave-size 1.2s linear infinite, blocks-wave-pos-9 1.2s linear infinite;
    animation-delay: 0.4s;
  }

  @keyframes blocks-wave-size {
    0%,
    50% {
      width: 7.33px;
      height: 7.33px;
    }

    25% {
      width: 1.33px;
      height: 1.33px;
    }
  }

  @keyframes blocks-wave-pos-1 {
    0%,
    50% {
      x: 1px;
      y: 1px;
    }

    25% {
      x: 4px;
      y: 4px;
    }
  }

  @keyframes blocks-wave-pos-2 {
    0%,
    50% {
      x: 8.33px;
      y: 1px;
    }

    25% {
      x: 11.33px;
      y: 4px;
    }
  }

  @keyframes blocks-wave-pos-3 {
    0%,
    50% {
      x: 1px;
      y: 8.33px;
    }

    25% {
      x: 4px;
      y: 11.33px;
    }
  }

  @keyframes blocks-wave-pos-4 {
    0%,
    50% {
      x: 15.66px;
      y: 1px;
    }

    25% {
      x: 18.66px;
      y: 4px;
    }
  }

  @keyframes blocks-wave-pos-5 {
    0%,
    50% {
      x: 8.33px;
      y: 8.33px;
    }

    25% {
      x: 11.33px;
      y: 11.33px;
    }
  }

  @keyframes blocks-wave-pos-6 {
    0%,
    50% {
      x: 1px;
      y: 15.66px;
    }

    25% {
      x: 4px;
      y: 18.66px;
    }
  }

  @keyframes blocks-wave-pos-7 {
    0%,
    50% {
      x: 15.66px;
      y: 8.33px;
    }

    25% {
      x: 18.66px;
      y: 11.33px;
    }
  }

  @keyframes blocks-wave-pos-8 {
    0%,
    50% {
      x: 8.33px;
      y: 15.66px;
    }

    25% {
      x: 11.33px;
      y: 18.66px;
    }
  }

  @keyframes blocks-wave-pos-9 {
    0%,
    50% {
      x: 15.66px;
      y: 15.66px;
    }

    25% {
      x: 18.66px;
      y: 18.66px;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .blocks-wave-spinner :global(rect) {
      animation: none;
    }
  }
</style>
