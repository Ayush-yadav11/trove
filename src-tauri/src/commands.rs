//! Commands invoked from the Svelte frontend. Engine work is CPU-bound and
//! runs on the blocking pool so the IPC thread stays responsive.

use crate::engine::FileResult;
use crate::{indexing, AppState};
use quillrag::StoreStats;
use serde::Serialize;
use std::path::PathBuf;
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

type CmdResult<T> = Result<T, String>;

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    folders: Vec<String>,
    shortcut: String,
    indexing: bool,
    stats: StoreStats,
}

fn snapshot(state: &AppState) -> CmdResult<Snapshot> {
    let settings = state.settings();
    Ok(Snapshot {
        folders: settings
            .folders
            .iter()
            .map(|f| f.display().to_string())
            .collect(),
        shortcut: settings.shortcut.clone(),
        indexing: state.engine.is_indexing(),
        stats: state.engine.stats().map_err(|e| format!("{e:#}"))?,
    })
}

#[tauri::command]
pub fn get_state(state: State<'_, AppState>) -> CmdResult<Snapshot> {
    snapshot(&state)
}

#[tauri::command]
pub fn add_folder(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<Snapshot> {
    let folder = std::path::absolute(PathBuf::from(&path)).map_err(err)?;
    if !folder.is_dir() {
        return Err(format!("{path} is not a folder"));
    }
    state.update_settings(|s| {
        if let Some(parent) = s.folders.iter().find(|f| folder.starts_with(f)) {
            return Err(format!("Already covered by {}", parent.display()));
        }
        // A new parent folder subsumes any of its subfolders already listed.
        s.folders.retain(|f| !f.starts_with(&folder));
        s.folders.push(folder.clone());
        Ok(())
    })?;
    indexing::start(&app);
    snapshot(&state)
}

#[tauri::command]
pub async fn remove_folder(
    app: AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> CmdResult<Snapshot> {
    let folders = state.update_settings(|s| {
        s.folders.retain(|f| f.display().to_string() != path);
        Ok(s.folders.clone())
    })?;
    // Drop the folder's documents right away rather than on the next pass,
    // unless a pass is running — it sweeps removed folders when it ends.
    if state.engine.is_indexing() {
        indexing::start(&app);
    } else {
        let engine = state.engine.clone();
        tauri::async_runtime::spawn_blocking(move || engine.retain_under(&folders))
            .await
            .map_err(err)?
            .map_err(|e| format!("{e:#}"))?;
    }
    snapshot(&state)
}

#[tauri::command]
pub fn reindex(app: AppHandle) {
    indexing::start(&app);
}

#[tauri::command]
pub async fn search(
    state: State<'_, AppState>,
    query: String,
    limit: Option<usize>,
) -> CmdResult<Vec<FileResult>> {
    let engine = state.engine.clone();
    let limit = limit.unwrap_or(12).min(50);
    tauri::async_runtime::spawn_blocking(move || engine.search(&query, limit))
        .await
        .map_err(err)?
        .map_err(|e| format!("{e:#}"))
}

/// Only paths Trove itself indexed may be opened, so the webview can't be
/// used to launch arbitrary files.
fn ensure_indexed(state: &AppState, path: &str) -> CmdResult<()> {
    match state.engine.is_indexed(path) {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("{path} is not in the index")),
        Err(e) => Err(format!("{e:#}")),
    }
}

#[tauri::command]
pub fn open_file(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<()> {
    ensure_indexed(&state, &path)?;
    app.opener().open_path(&path, None::<&str>).map_err(err)
}

#[tauri::command]
pub fn reveal_file(app: AppHandle, state: State<'_, AppState>, path: String) -> CmdResult<()> {
    ensure_indexed(&state, &path)?;
    app.opener().reveal_item_in_dir(&path).map_err(err)
}
