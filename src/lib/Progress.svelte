<script lang="ts">
  import type { IndexStatus } from "./api";

  let { status, compact = false }: { status: IndexStatus; compact?: boolean } = $props();

  const pct = $derived(status.total ? Math.min(100, (status.done / status.total) * 100) : 0);
  const label = $derived(
    status.phase === "scanning"
      ? `Scanning ${status.done.toLocaleString()} / ${status.total.toLocaleString()} files`
      : status.phase === "embedding"
        ? `Reading ${status.done.toLocaleString()} / ${status.total.toLocaleString()} passages`
        : "Saving index",
  );
  const folderName = $derived(status.folder.split(/[\\/]/).filter(Boolean).pop() ?? status.folder);
</script>

<div class="progress" class:compact>
  <div class="line">
    <span>
      {label}
      {#if status.folderCount > 1}
        <span class="muted">· folder {status.folderIndex + 1} of {status.folderCount}</span>
      {/if}
    </span>
    <span class="muted folder" title={status.folder}>{folderName}</span>
  </div>
  <div
    class="track"
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(pct)}
  >
    <div class="fill" style:width="{pct}%"></div>
  </div>
</div>

<style>
  .line {
    display: flex;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 6px;
    font-size: 13px;
  }
  .compact .line {
    margin-bottom: 4px;
    font-size: 12px;
  }
  .muted {
    color: var(--text-3);
  }
  .folder {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .track {
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--surface-2);
  }
  .compact .track {
    height: 3px;
  }
  .fill {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s ease-out;
  }
</style>
