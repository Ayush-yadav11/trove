<script lang="ts">
  import { api, errorText, type IndexStatus, type IndexSummary, type Snapshot } from "./api";
  import { formatBytes } from "./snippet";
  import Progress from "./Progress.svelte";
  import WindowControls from "./WindowControls.svelte";

  let {
    snapshot,
    progress,
    summary,
    lastError,
    onBack,
    onAddFolder,
    onSnapshot,
  }: {
    snapshot: Snapshot | null;
    progress: IndexStatus | null;
    summary: IndexSummary | null;
    lastError: string;
    onBack: () => void;
    onAddFolder: () => void;
    onSnapshot: (s: Snapshot) => void;
  } = $props();

  let error = $state("");
  let removing = $state<string | null>(null);
  let showFailed = $state(false);

  const types = $derived(
    Object.entries(snapshot?.stats.by_extension ?? {}).sort((a, b) => b[1] - a[1]),
  );

  async function remove(folder: string) {
    removing = folder;
    error = "";
    try {
      onSnapshot(await api.removeFolder(folder));
    } catch (e) {
      error = errorText(e);
    } finally {
      removing = null;
    }
  }

  async function reindex() {
    error = "";
    try {
      await api.reindex();
    } catch (e) {
      error = errorText(e);
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.preventDefault();
      onBack();
    }
  }
</script>

<svelte:window onkeydown={onKeydown} />

<div class="settings">
  <header data-tauri-drag-region>
    <button class="back" onclick={onBack} aria-label="Back to search">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 5l-7 7 7 7" /></svg>
    </button>
    <h1 data-tauri-drag-region>Folders &amp; settings</h1>
    <WindowControls />
  </header>

  <main>
    {#if error || lastError}
      <p class="error" role="alert">{error || lastError}</p>
    {/if}

    <section>
      <div class="section-head">
        <h2>Indexed folders</h2>
        <button class="secondary" onclick={onAddFolder}>Add folder</button>
      </div>
      {#if snapshot?.folders.length}
        <ul class="folders">
          {#each snapshot.folders as folder (folder)}
            <li>
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
              </svg>
              <span class="path" title={folder}>{folder}</span>
              <button
                class="remove"
                onclick={() => remove(folder)}
                disabled={removing === folder}
                aria-label="Stop indexing {folder}"
              >
                {removing === folder ? "Removing…" : "Remove"}
              </button>
            </li>
          {/each}
        </ul>
        <p class="note">
          Subfolders are included. Hidden folders and <code>node_modules</code>, <code>target</code>,
          <code>dist</code>, <code>build</code> and <code>venv</code> are skipped.
        </p>
        <p class="note">
          Indexes notes, documents and code (<code>.md</code>, <code>.txt</code>, <code>.pdf</code>,
          source files and more). Text in screenshots, photos and scanned PDFs is read with on-device
          OCR (English only), which is slower: expect a few seconds per image or scanned page the
          first time.
        </p>
      {:else}
        <p class="note">No folders yet. Add one to start searching.</p>
      {/if}
    </section>

    <section>
      <div class="section-head">
        <h2>Index</h2>
        <button class="secondary" onclick={reindex} disabled={!snapshot?.folders.length}>
          {snapshot?.indexing ? "Queue re-index" : "Re-index now"}
        </button>
      </div>

      {#if progress}
        <div class="card"><Progress status={progress} /></div>
      {:else if summary}
        <div class="card summary">
          <p>
            Up to date. Last pass took {summary.seconds < 1 ? "<1" : summary.seconds.toFixed(0)}s:
            {summary.indexed.toLocaleString()} new or changed,
            {summary.unchanged.toLocaleString()} unchanged{#if summary.removed},
              {summary.removed.toLocaleString()} removed{/if}.
          </p>
          {#each summary.errors as e}
            <p class="warn">{e}</p>
          {/each}
          {#if summary.failed.length}
            <button class="link" onclick={() => (showFailed = !showFailed)}>
              {summary.failed.length} file{summary.failed.length === 1 ? "" : "s"} skipped
              {showFailed ? "▴" : "▾"}
            </button>
            {#if showFailed}
              <ul class="failed">
                {#each summary.failed as f}<li>{f}</li>{/each}
              </ul>
            {/if}
          {/if}
        </div>
      {/if}

      {#if snapshot}
        <dl class="stats">
          <div><dt>Files</dt><dd>{snapshot.stats.documents.toLocaleString()}</dd></div>
          <div><dt>Passages</dt><dd>{snapshot.stats.chunks.toLocaleString()}</dd></div>
          <div><dt>Source size</dt><dd>{formatBytes(snapshot.stats.total_source_bytes)}</dd></div>
        </dl>
        {#if types.length}
          <div class="types">
            {#each types as [ext, n]}
              <span class="chip">.{ext} <b>{n.toLocaleString()}</b></span>
            {/each}
          </div>
        {/if}
      {/if}
    </section>

    <section>
      <h2>Shortcut</h2>
      <p class="note">
        Press <kbd>{snapshot?.shortcut ?? "…"}</kbd> anywhere to open Trove. Closing the window keeps
        Trove running in the tray, where you can also quit it.
      </p>
    </section>

    <section>
      <h2>Privacy</h2>
      <p class="note">
        Trove makes no network requests. The search model is built into the app, and the index is
        stored only on this computer.
      </p>
    </section>
  </main>
</div>

<style>
  .settings {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }
  header {
    --header-pad-y: 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: var(--header-pad-y) 0 var(--header-pad-y) 14px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  h1 {
    flex: 1;
    margin: 0;
    font-size: 15px;
    font-weight: 600;
  }
  .back {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
  }
  .back:hover {
    background: var(--surface-2);
  }
  .back svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: var(--text-2);
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }

  main {
    flex: 1;
    overflow-y: auto;
    padding: 4px 20px 24px;
  }
  section {
    padding: 16px 0;
    border-bottom: 1px solid var(--border);
  }
  section:last-child {
    border-bottom: 0;
  }
  .section-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 10px;
  }
  h2 {
    margin: 0 0 8px;
    font-size: 13px;
    font-weight: 600;
    color: var(--text-2);
    letter-spacing: 0.02em;
  }
  .section-head h2 {
    margin: 0;
  }

  .secondary {
    padding: 6px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--surface);
    font-size: 13px;
    box-shadow: var(--shadow);
  }
  .secondary:hover:not(:disabled) {
    background: var(--surface-2);
  }
  button:disabled {
    cursor: default;
    opacity: 0.55;
  }

  .folders {
    margin: 0;
    padding: 0;
    list-style: none;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .folders li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 9px 10px 9px 12px;
  }
  .folders li + li {
    border-top: 1px solid var(--border);
  }
  .folders svg {
    width: 16px;
    height: 16px;
    flex: none;
    fill: var(--accent-soft);
    stroke: var(--accent);
    stroke-width: 1.6;
  }
  .path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    user-select: text;
  }
  .remove {
    padding: 4px 8px;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text-3);
    font-size: 12px;
  }
  .remove:hover:not(:disabled) {
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
  }

  .note {
    margin: 10px 0 0;
    color: var(--text-3);
    font-size: 12.5px;
  }
  section > h2 + .note {
    margin-top: 0;
  }
  code,
  kbd {
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface-2);
    font: 11.5px var(--mono);
  }

  .card {
    padding: 12px 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--surface);
  }
  .summary p {
    margin: 0;
    color: var(--text-2);
    font-size: 13px;
  }
  .summary .warn {
    margin-top: 6px;
    color: var(--danger);
  }
  .link {
    margin-top: 6px;
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    font-size: 12.5px;
  }
  .failed {
    max-height: 140px;
    margin: 6px 0 0;
    padding: 0 0 0 16px;
    overflow-y: auto;
    color: var(--text-3);
    font-size: 12px;
    user-select: text;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    margin: 12px 0 0;
  }
  .stats div {
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  dt {
    color: var(--text-3);
    font-size: 12px;
  }
  dd {
    margin: 2px 0 0;
    font-size: 17px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .types {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 10px;
  }
  .chip {
    padding: 2px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--text-2);
    font-size: 12px;
  }
  .chip b {
    font-weight: 600;
    color: var(--text);
  }

  .error {
    margin: 12px 0 0;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
    user-select: text;
  }
</style>
