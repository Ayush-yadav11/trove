// Typed bindings for the Rust commands and events in src-tauri.

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export interface StoreStats {
  documents: number;
  chunks: number;
  total_source_bytes: number;
  by_extension: Record<string, number>;
}

export interface Snapshot {
  folders: string[];
  shortcut: string;
  indexing: boolean;
  stats: StoreStats;
}

export interface FileResult {
  path: string;
  name: string;
  folder: string;
  ext: string;
  snippet: string;
  /** Page of the snippet, for PDFs. */
  page: number | null;
  /** Other pages of the same file that also matched, best first. */
  otherPages: number[];
  extraMatches: number;
}

export interface IndexStatus {
  folder: string;
  folderIndex: number;
  folderCount: number;
  phase: "scanning" | "embedding" | "finishing";
  done: number;
  total: number;
}

export interface IndexSummary {
  indexed: number;
  unchanged: number;
  removed: number;
  failed: string[];
  errors: string[];
  seconds: number;
}

/** `Ok` or `Err` from the Rust `Result<IndexSummary, String>`. */
export type IndexDone = { Ok: IndexSummary } | { Err: string };

export type View = "search" | "settings";

export const api = {
  getState: () => invoke<Snapshot>("get_state"),
  addFolder: (path: string) => invoke<Snapshot>("add_folder", { path }),
  removeFolder: (path: string) => invoke<Snapshot>("remove_folder", { path }),
  reindex: () => invoke<void>("reindex"),
  search: (query: string, limit = 12) => invoke<FileResult[]>("search", { query, limit }),
  openFile: (path: string) => invoke<void>("open_file", { path }),
  revealFile: (path: string) => invoke<void>("reveal_file", { path }),
};

export const events = {
  onProgress: (f: (s: IndexStatus) => void): Promise<UnlistenFn> =>
    listen<IndexStatus>("trove://index-progress", (e) => f(e.payload)),
  onDone: (f: (d: IndexDone) => void): Promise<UnlistenFn> =>
    listen<IndexDone>("trove://index-done", (e) => f(e.payload)),
  onNavigate: (f: (v: View) => void): Promise<UnlistenFn> =>
    listen<View>("trove://navigate", (e) => f(e.payload)),
};

export function errorText(e: unknown): string {
  return typeof e === "string" ? e : e instanceof Error ? e.message : String(e);
}
