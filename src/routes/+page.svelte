<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { open } from "@tauri-apps/plugin-dialog";
  import {
    api,
    events,
    errorText,
    type IndexStatus,
    type IndexSummary,
    type Snapshot,
    type View,
  } from "$lib/api";
  import Search from "$lib/Search.svelte";
  import Settings from "$lib/Settings.svelte";

  let view = $state<View>("search");
  let snapshot = $state<Snapshot | null>(null);
  let progress = $state<IndexStatus | null>(null);
  let summary = $state<IndexSummary | null>(null);
  let lastError = $state("");
  let focusSignal = $state(0);
  let refreshSignal = $state(0);

  async function refresh() {
    try {
      snapshot = await api.getState();
    } catch (e) {
      lastError = errorText(e);
    }
  }

  async function addFolder() {
    try {
      const picked = await open({ directory: true, multiple: false, title: "Choose a folder for Trove to index" });
      if (typeof picked === "string") {
        lastError = "";
        snapshot = await api.addFolder(picked);
        view = "settings";
      }
    } catch (e) {
      lastError = errorText(e);
      view = "settings";
    } finally {
      await getCurrentWindow().setFocus();
    }
  }

  onMount(() => {
    refresh();
    const win = getCurrentWindow();
    const subs = [
      events.onProgress((s) => {
        progress = s;
        if (snapshot) snapshot.indexing = true;
      }),
      events.onDone((d) => {
        progress = null;
        if ("Ok" in d) {
          summary = d.Ok;
          lastError = "";
        } else {
          lastError = d.Err;
        }
        refresh();
        refreshSignal++;
      }),
      events.onNavigate((v) => {
        view = v;
        focusSignal++;
      }),
      win.onFocusChanged(({ payload: focused }) => {
        if (focused) focusSignal++;
      }),
    ];
    return () => subs.forEach((p) => p.then((unlisten) => unlisten()));
  });
</script>

{#if view === "search"}
  <Search
    {snapshot}
    {progress}
    {focusSignal}
    {refreshSignal}
    onSettings={() => (view = "settings")}
    onAddFolder={addFolder}
  />
{:else}
  <Settings
    {snapshot}
    {progress}
    {summary}
    {lastError}
    onBack={() => {
      view = "search";
      focusSignal++;
    }}
    onAddFolder={addFolder}
    onSnapshot={(s) => (snapshot = s)}
  />
{/if}
