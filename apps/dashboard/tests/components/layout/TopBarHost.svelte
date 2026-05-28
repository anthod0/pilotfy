<script lang="ts">
  import * as Sidebar from '../../../src/lib/components/ui/sidebar/index.js'
  import TopBar from '../../../src/components/layout/TopBar.svelte'
</script>

<Sidebar.Provider>
  <TopBar />
</Sidebar.Provider>
