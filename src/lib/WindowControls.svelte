<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  const win = getCurrentWindow();
  let maximized = $state(false);

  onMount(() => {
    const sync = async () => {
      maximized = await win.isMaximized();
    };
    sync();
    const unlisten = win.onResized(sync);
    return () => {
      unlisten.then((f) => f());
    };
  });
</script>

<div class="controls">
  <button onclick={() => win.minimize()} title="Minimize" aria-label="Minimize">
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1 5h8" /></svg>
  </button>
  <button
    onclick={() => win.toggleMaximize()}
    title={maximized ? "Restore" : "Maximize"}
    aria-label={maximized ? "Restore" : "Maximize"}
  >
    {#if maximized}
      <svg viewBox="0 0 10 10" aria-hidden="true">
        <path d="M3 1.5h5.5V7M1.5 3h5.5v5.5H1.5z" />
      </svg>
    {:else}
      <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5h7v7h-7z" /></svg>
    {/if}
  </button>
  <!-- Closing hides to the tray (see on_window_event in lib.rs). -->
  <button class="close" onclick={() => win.close()} title="Close to tray" aria-label="Close">
    <svg viewBox="0 0 10 10" aria-hidden="true"><path d="M1.5 1.5l7 7M8.5 1.5l-7 7" /></svg>
  </button>
</div>

<style>
  .controls {
    display: flex;
    align-self: stretch;
    /* Fill the header's full height, flush with the window's top-right edge. */
    margin: calc(var(--header-pad-y) * -1) 0 calc(var(--header-pad-y) * -1) 4px;
  }
  button {
    display: grid;
    place-items: center;
    width: 46px;
    border: 0;
    background: transparent;
  }
  button:hover {
    background: var(--surface-2);
  }
  button:active {
    background: var(--border);
  }
  .close:hover {
    background: #c42b1c;
  }
  .close:hover svg {
    stroke: #fff;
  }
  .close:active {
    background: #b0281a;
  }
  svg {
    width: 10px;
    height: 10px;
    fill: none;
    stroke: var(--text-2);
    stroke-width: 1;
    shape-rendering: crispEdges;
  }
  .close svg {
    shape-rendering: geometricPrecision;
  }
</style>
