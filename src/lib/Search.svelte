<script lang="ts">
  import { untrack } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api, errorText, type FileResult, type IndexStatus, type Snapshot } from "./api";
  import { kind, snippet } from "./snippet";
  import Progress from "./Progress.svelte";
  import WindowControls from "./WindowControls.svelte";

  let {
    snapshot,
    progress,
    focusSignal,
    refreshSignal,
    onSettings,
    onAddFolder,
  }: {
    snapshot: Snapshot | null;
    progress: IndexStatus | null;
    focusSignal: number;
    refreshSignal: number;
    onSettings: () => void;
    onAddFolder: () => void;
  } = $props();

  let input: HTMLInputElement | undefined = $state();
  let list: HTMLUListElement | undefined = $state();
  let query = $state("");
  let results = $state<FileResult[]>([]);
  let selected = $state(0);
  let loading = $state(false);
  let error = $state("");
  let searched = $state(""); // query the current results belong to

  let seq = 0;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const hasFolders = $derived((snapshot?.folders.length ?? 0) > 0);

  async function run() {
    const mine = ++seq;
    const q = query;
    if (!q.trim()) {
      results = [];
      searched = "";
      loading = false;
      error = "";
      return;
    }
    loading = true;
    try {
      const r = await api.search(q);
      if (mine !== seq) return; // a newer keystroke superseded this one
      results = r;
      searched = q;
      selected = 0;
      error = "";
    } catch (e) {
      if (mine === seq) error = errorText(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  function onInput() {
    clearTimeout(timer);
    timer = setTimeout(run, 120);
  }

  // Window re-shown via shortcut/tray: ready to type, previous query selected.
  $effect(() => {
    void focusSignal;
    input?.focus();
    input?.select();
  });

  // Index changed underneath us: refresh the visible results.
  $effect(() => {
    void refreshSignal;
    untrack(() => {
      if (query.trim()) run();
    });
  });

  $effect(() => {
    const row = list?.children[selected] as HTMLElement | undefined;
    row?.scrollIntoView({ block: "nearest" });
  });

  async function open(r: FileResult, reveal = false) {
    try {
      await (reveal ? api.revealFile(r.path) : api.openFile(r.path));
      await getCurrentWindow().hide();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      if (!results.length) return;
      const step = e.key === "ArrowDown" ? 1 : -1;
      selected = (selected + step + results.length) % results.length;
    } else if (e.key === "Enter" && results[selected]) {
      e.preventDefault();
      open(results[selected], e.ctrlKey || e.metaKey);
    } else if (e.key === "Escape") {
      e.preventDefault();
      if (query) {
        query = "";
        run();
      } else {
        await getCurrentWindow().hide();
      }
    }
  }
</script>

<div class="search">
  <header data-tauri-drag-region>
    <svg class="glass" viewBox="0 0 24 24" aria-hidden="true">
      <circle cx="11" cy="11" r="6.5" />
      <path d="m16 16 4.5 4.5" />
    </svg>
    <label class="sr-only" for="q">Search your files</label>
    <input
      id="q"
      bind:this={input}
      bind:value={query}
      oninput={onInput}
      onkeydown={onKeydown}
      placeholder={hasFolders ? "Search your files by meaning…" : "Add a folder to start searching"}
      disabled={!hasFolders}
      autocomplete="off"
      spellcheck="false"
      role="combobox"
      aria-expanded={results.length > 0}
      aria-controls="results"
      aria-activedescendant={results.length ? `r-${selected}` : undefined}
    />
    {#if loading}<span class="spinner" aria-label="Searching"></span>{/if}
    <button class="icon" onclick={onSettings} title="Folders & settings" aria-label="Folders and settings">
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 7h10M18 7h2M4 17h4M12 17h8" />
        <circle cx="16" cy="7" r="2" />
        <circle cx="10" cy="17" r="2" />
      </svg>
    </button>
    <WindowControls />
  </header>

  {#if progress}
    <div class="progress-strip"><Progress status={progress} compact /></div>
  {/if}

  <main>
    {#if error}
      <p class="error" role="alert">{error}</p>
    {/if}

    {#if !snapshot}
      <p class="empty">Starting…</p>
    {:else if !hasFolders}
      <div class="welcome">
        <h1>Find anything you've saved, by what it's about.</h1>
        <p>
          Trove indexes your notes, documents and code, and searches them by meaning, not just
          keywords. Everything stays on this computer.
        </p>
        <button class="primary" onclick={onAddFolder}>Add a folder</button>
      </div>
    {:else if results.length}
      <ul id="results" role="listbox" bind:this={list} aria-label="Results">
        {#each results as r, i (r.path)}
          <li
            id="r-{i}"
            role="option"
            aria-selected={i === selected}
            class:selected={i === selected}
            onmousemove={() => (selected = i)}
            onclick={() => open(r)}
            onkeydown={() => {}}
          >
            <span class="badge" data-kind={kind(r.ext)}>{kind(r.ext)}</span>
            <div class="body">
              <div class="title">
                <span class="name">{r.name}</span>
                {#if r.page}
                  <span class="page">p. {r.page}</span>
                {/if}
                {#if r.otherPages.length}
                  <span class="more">
                    also p. {r.otherPages.slice(0, 4).join(", ")}{r.otherPages.length > 4 ? "…" : ""}
                  </span>
                {:else if r.extraMatches}
                  <span class="more">+{r.extraMatches} more</span>
                {/if}
              </div>
              <div class="folder"><span dir="ltr">{r.folder}</span></div>
              <p class="snippet">
                {#each snippet(r.snippet, searched) as seg}{#if seg.hit}<mark>{seg.text}</mark>{:else}{seg.text}{/if}{/each}
              </p>
            </div>
            <button
              class="reveal"
              title="Show in folder (Ctrl+Enter)"
              aria-label="Show {r.name} in folder"
              onclick={(e) => {
                e.stopPropagation();
                open(r, true);
              }}
            >
              <svg viewBox="0 0 24 24" aria-hidden="true">
                <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2Z" />
              </svg>
            </button>
          </li>
        {/each}
      </ul>
    {:else if searched && !loading}
      <p class="empty">No matches for “{searched}”.</p>
    {:else if !query}
      <div class="hint">
        {#if snapshot.stats.documents === 0 && snapshot.indexing}
          <p>Indexing your files for the first time. Results appear as each folder finishes.</p>
        {:else}
          <p>
            Searching <strong>{snapshot.stats.documents.toLocaleString()}</strong> files in
            {snapshot.folders.length}
            {snapshot.folders.length === 1 ? "folder" : "folders"}.
          </p>
        {/if}
        <p class="tip">Describe what you're looking for, e.g. <em>"notes on the database migration plan"</em>.</p>
      </div>
    {/if}
  </main>

  <footer>
    <span><kbd>↑</kbd><kbd>↓</kbd> move</span>
    <span><kbd>Enter</kbd> open</span>
    <span><kbd>Ctrl</kbd><kbd>Enter</kbd> show in folder</span>
    <span><kbd>Esc</kbd> {query ? "clear" : "hide"}</span>
    {#if snapshot}<span class="shortcut">{snapshot.shortcut}</span>{/if}
  </footer>
</div>

<style>
  .search {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  header {
    --header-pad-y: 14px;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: var(--header-pad-y) 0 var(--header-pad-y) 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }
  .glass {
    width: 20px;
    height: 20px;
    flex: none;
    fill: none;
    stroke: var(--text-3);
    stroke-width: 2;
    stroke-linecap: round;
    pointer-events: none;
  }
  input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: transparent;
    color: var(--text);
    font: 400 19px/1.3 var(--font);
    outline: none;
    user-select: text;
  }
  input::placeholder {
    color: var(--text-3);
  }
  .icon,
  .reveal {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
  }
  .icon:hover,
  .reveal:hover {
    background: var(--surface-2);
  }
  .icon svg,
  .reveal svg {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: var(--text-2);
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .progress-strip {
    padding: 6px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  main {
    flex: 1;
    overflow-y: auto;
    padding: 6px;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  li {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 10px 10px 10px 12px;
    border-radius: var(--radius);
    cursor: pointer;
  }
  li.selected {
    background: var(--selected);
  }
  .badge {
    flex: none;
    width: 40px;
    margin-top: 2px;
    padding: 3px 0;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-2);
    font: 600 10px/1.2 var(--font);
    letter-spacing: 0.04em;
    text-align: center;
  }
  .badge[data-kind="PDF"] {
    background: #fde2df;
    color: #9b2317;
  }
  .badge[data-kind="MD"],
  .badge[data-kind="TXT"] {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .badge[data-kind="IMG"] {
    background: #dcecfb;
    color: #1b5a8f;
  }
  @media (prefers-color-scheme: dark) {
    .badge[data-kind="PDF"] {
      background: #45211d;
      color: #f5a39a;
    }
    .badge[data-kind="IMG"] {
      background: #1c3246;
      color: #9ccaf2;
    }
  }
  .body {
    flex: 1;
    min-width: 0;
  }
  .title {
    display: flex;
    align-items: baseline;
    gap: 8px;
  }
  .name {
    overflow: hidden;
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .page {
    flex: none;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 11.5px;
    font-variant-numeric: tabular-nums;
  }
  .more {
    flex: none;
    color: var(--text-3);
    font-size: 12px;
  }
  .folder {
    overflow: hidden;
    color: var(--text-3);
    font-size: 12px;
    text-overflow: ellipsis;
    white-space: nowrap;
    /* Truncate long paths from the left so the nearest folders stay visible. */
    direction: rtl;
    text-align: left;
  }
  .snippet {
    display: -webkit-box;
    margin: 4px 0 0;
    overflow: hidden;
    color: var(--text-2);
    font-size: 13px;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  mark {
    padding: 0 1px;
    border-radius: 3px;
    background: var(--mark);
    color: var(--mark-text);
  }
  .reveal {
    opacity: 0;
  }
  li.selected .reveal,
  .reveal:focus-visible {
    opacity: 1;
  }

  .welcome {
    max-width: 440px;
    margin: 56px auto 0;
    padding: 0 16px;
    text-align: center;
  }
  .welcome h1 {
    margin: 0 0 10px;
    font-size: 20px;
    font-weight: 600;
    line-height: 1.3;
    text-wrap: balance;
  }
  .welcome p {
    margin: 0 0 22px;
    color: var(--text-2);
  }
  .primary {
    padding: 9px 18px;
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--accent);
    color: #fff;
    font-weight: 600;
  }
  @media (prefers-color-scheme: dark) {
    .primary {
      color: #1d1b16;
    }
  }

  .hint,
  .empty {
    padding: 28px 16px;
    color: var(--text-2);
    text-align: center;
  }
  .hint p {
    margin: 0 0 8px;
  }
  .tip {
    color: var(--text-3);
    font-size: 13px;
  }
  .error {
    margin: 6px;
    padding: 8px 12px;
    border-radius: var(--radius-sm);
    background: color-mix(in srgb, var(--danger) 12%, transparent);
    color: var(--danger);
    user-select: text;
  }

  footer {
    display: flex;
    gap: 16px;
    padding: 8px 18px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    color: var(--text-3);
    font-size: 12px;
  }
  .shortcut {
    margin-left: auto;
  }
  kbd {
    display: inline-block;
    min-width: 18px;
    margin-right: 3px;
    padding: 0 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--surface-2);
    font: 11px/16px var(--font);
    text-align: center;
  }
  @media (max-width: 620px) {
    footer span:nth-child(3) {
      display: none;
    }
  }
</style>
