//! Trove's wrapper around the quillrag engine: one store + BM25 sidecar
//! shared by every command, and two lazily loaded embedders so a long index
//! pass never blocks a search. The model file is mmap'd, so the second
//! embedder shares its pages instead of doubling memory.

use anyhow::{Context, Result};
use quillrag::indexer::index_directory_with;
use quillrag::{
    hybrid_search_with, Embedder, Extractor, IndexHooks, IndexOptions, IndexProgress,
    SearchOptions, Store, StoreStats, TantivyIndex,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering::SeqCst};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::Instant;

/// Meaning-based matches below this cosine similarity are dropped. Measured
/// on sample notes: right answers scored 0.30-0.57, unrelated files <= 0.21.
/// Keyword matches are kept regardless.
const MIN_SIMILARITY: f32 = 0.25;

pub struct Engine {
    store: Store,
    bm25: TantivyIndex,
    model_dir: PathBuf,
    /// First-run model materialization writes files; never race two loads.
    load_lock: Mutex<()>,
    search_embedder: Mutex<Option<Embedder>>,
    index_embedder: Mutex<Option<Embedder>>,
    /// An index worker owns the slot.
    indexing: AtomicBool,
    /// Another pass was requested while one was running.
    rerun: AtomicBool,
}

/// One file in the results list; its best-matching chunk is the snippet.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub ext: String,
    pub snippet: String,
    /// Page of the snippet, for paged formats like PDF.
    pub page: Option<u32>,
    /// Other pages of the same file that also matched, best first.
    pub other_pages: Vec<u32>,
    /// Other chunks of the same file that also matched.
    pub extra_matches: usize,
}

/// Progress of the whole multi-folder pass, as sent to the UI.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexStatus {
    pub folder: String,
    pub folder_index: usize,
    pub folder_count: usize,
    /// "scanning" | "embedding" | "finishing"
    pub phase: &'static str,
    pub done: usize,
    pub total: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexSummary {
    pub indexed: usize,
    pub unchanged: usize,
    pub removed: usize,
    /// Files that could not be read or had no text, with the reason.
    pub failed: Vec<String>,
    /// Folders that could not be walked at all.
    pub errors: Vec<String>,
    pub seconds: f64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    // A panic mid-embed leaves the model usable; don't brick the app.
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Engine {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let index_dir = data_dir.join("index");
        Ok(Self {
            store: Store::open(&index_dir).context("opening index")?,
            bm25: TantivyIndex::open(&index_dir).context("opening keyword index")?,
            model_dir: data_dir.join("model"),
            load_lock: Mutex::new(()),
            search_embedder: Mutex::new(None),
            index_embedder: Mutex::new(None),
            indexing: AtomicBool::new(false),
            rerun: AtomicBool::new(false),
        })
    }

    fn with_embedder<T>(
        &self,
        slot: &Mutex<Option<Embedder>>,
        f: impl FnOnce(&mut Embedder) -> Result<T>,
    ) -> Result<T> {
        let mut guard = lock(slot);
        if guard.is_none() {
            let _load = lock(&self.load_lock);
            *guard = Some(Embedder::load(&self.model_dir).context("loading embedding model")?);
        }
        f(guard.as_mut().expect("embedder loaded above"))
    }

    /// Load the search model ahead of the first keystroke.
    pub fn warm_up(&self) -> Result<()> {
        self.with_embedder(&self.search_embedder, |_| Ok(()))
    }

    pub fn stats(&self) -> Result<StoreStats> {
        self.store.stats()
    }

    pub fn is_indexed(&self, path: &str) -> Result<bool> {
        Ok(self.store.list_documents()?.contains_key(path))
    }

    /// Hybrid search, grouped so each file appears once at its best rank.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<FileResult>> {
        let query = query.trim();
        if query.is_empty() || limit == 0 {
            return Ok(Vec::new());
        }
        let options = SearchOptions {
            min_similarity: MIN_SIMILARITY,
        };
        let hits = self.with_embedder(&self.search_embedder, |emb| {
            hybrid_search_with(query, limit * 3, &self.store, &self.bm25, emb, options)
        })?;

        let mut out: Vec<FileResult> = Vec::new();
        for hit in hits {
            if let Some(existing) = out.iter_mut().find(|r| r.path == hit.path) {
                existing.extra_matches += 1;
                if let Some(p) = hit.page {
                    if existing.page != Some(p) && !existing.other_pages.contains(&p) {
                        existing.other_pages.push(p);
                    }
                }
                continue;
            }
            if out.len() == limit {
                continue;
            }
            let p = Path::new(&hit.path);
            out.push(FileResult {
                name: p
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| hit.path.clone()),
                folder: p
                    .parent()
                    .map(|d| d.display().to_string())
                    .unwrap_or_default(),
                ext: p
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default(),
                snippet: hit.text,
                page: hit.page,
                other_pages: Vec::new(),
                extra_matches: 0,
                path: hit.path,
            });
        }
        Ok(out)
    }

    /// Claim the indexing slot. Returns true when the caller must run the
    /// passes (see [`Engine::finish_pass`]); false means a running worker
    /// will pick the request up.
    pub fn request_index(&self) -> bool {
        self.rerun.store(true, SeqCst);
        !self.indexing.swap(true, SeqCst)
    }

    pub fn is_indexing(&self) -> bool {
        self.indexing.load(SeqCst)
    }

    /// Release the slot after a pass. Returns true when another pass was
    /// requested meanwhile and this worker re-claimed the slot to run it.
    pub fn finish_pass(&self) -> bool {
        self.indexing.store(false, SeqCst);
        self.rerun.load(SeqCst) && !self.indexing.swap(true, SeqCst)
    }

    /// One incremental pass over every folder, then drop anything indexed
    /// from folders the user has since removed. Only call while holding the
    /// slot from [`Engine::request_index`].
    pub fn index_pass(
        &self,
        folders: &[PathBuf],
        extractors: &[&dyn Extractor],
        options: IndexOptions,
        on_status: &(dyn Fn(IndexStatus) + Sync),
    ) -> Result<IndexSummary> {
        self.rerun.store(false, SeqCst);
        let started = Instant::now();
        let mut summary = IndexSummary::default();

        self.with_embedder(&self.index_embedder, |emb| {
            for (i, folder) in folders.iter().enumerate() {
                let folder_label = folder.display().to_string();
                let progress = |p: IndexProgress| {
                    let (phase, done, total) = match p {
                        IndexProgress::Discovered { files } => ("scanning", 0, files),
                        IndexProgress::Scanned { done, total } => ("scanning", done, total),
                        IndexProgress::Embedding { done, total } => ("embedding", done, total),
                        IndexProgress::Finished => ("finishing", 1, 1),
                    };
                    on_status(IndexStatus {
                        folder: folder_label.clone(),
                        folder_index: i,
                        folder_count: folders.len(),
                        phase,
                        done,
                        total,
                    });
                };
                let hooks = IndexHooks {
                    extractors,
                    progress: Some(&progress),
                };
                match index_directory_with(
                    folder,
                    &[],
                    &self.store,
                    &self.bm25,
                    emb,
                    options,
                    &hooks,
                ) {
                    Ok(r) => {
                        summary.indexed += r.indexed.len();
                        summary.unchanged += r.skipped_unchanged;
                        summary.removed += r.removed_missing;
                        summary.failed.extend(r.failed);
                    }
                    // One unreadable folder (unplugged drive, revoked access)
                    // must not stop the others.
                    Err(e) => summary.errors.push(format!("{folder_label}: {e:#}")),
                }
            }
            Ok(())
        })?;

        summary.removed += self.retain_under(folders)?;
        summary.seconds = started.elapsed().as_secs_f64();
        Ok(summary)
    }

    /// Delete every indexed document that is not inside one of `folders`.
    pub fn retain_under(&self, folders: &[PathBuf]) -> Result<usize> {
        let mut removed = 0;
        for path in self.store.list_documents()?.into_keys() {
            let p = Path::new(&path);
            if !folders.iter().any(|f| p.starts_with(f)) && self.store.delete_document(&path)? {
                removed += 1;
            }
        }
        if removed > 0 {
            self.bm25.rebuild_from(&self.store)?;
        }
        Ok(removed)
    }
}
