//! Background index worker. Requests coalesce: asking while a pass runs
//! schedules exactly one follow-up pass with the then-current folder list.

use crate::engine::IndexStatus;
use crate::settings::EXTRACTOR_LEVEL;
use crate::AppState;
use quillrag::{Extractor, ImageExtractor, IndexOptions, Ocr, PdfExtractor};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager};

pub const PROGRESS_EVENT: &str = "trove://index-progress";
pub const DONE_EVENT: &str = "trove://index-done";

/// Scan progress fires per file; cap UI updates to ~12/s.
const EMIT_EVERY: Duration = Duration::from_millis(80);

pub fn start(app: &AppHandle) {
    let state = app.state::<AppState>();
    if !state.engine.request_index() {
        return;
    }
    let app = app.clone();
    std::thread::Builder::new()
        .name("trove-index".into())
        .spawn(move || run(app))
        .expect("spawning index thread");
}

fn run(app: AppHandle) {
    let state = app.state::<AppState>();
    // Without OCR (models failed to load) text PDFs and notes still work.
    let ocr = match Ocr::bundled() {
        Ok(ocr) => Some(Arc::new(ocr)),
        Err(e) => {
            tracing::error!("OCR unavailable: {e:#}");
            None
        }
    };
    let pdf = match &ocr {
        Some(ocr) => PdfExtractor::default().with_ocr(ocr.clone()),
        None => PdfExtractor::default(),
    };
    let images = ocr.map(ImageExtractor::new);
    let mut extractors: Vec<&dyn Extractor> = vec![&pdf];
    if let Some(images) = &images {
        extractors.push(images);
    }
    loop {
        let (folders, level) = {
            let settings = state.settings();
            (settings.folders.clone(), settings.extractor_level)
        };
        let options = IndexOptions {
            retry_empty: level < EXTRACTOR_LEVEL,
            ..Default::default()
        };
        let last_emit: Mutex<Option<(Instant, &'static str)>> = Mutex::new(None);
        let on_status = |s: IndexStatus| {
            let mut last = last_emit.lock().unwrap();
            let due = match *last {
                None => true,
                Some((at, phase)) => {
                    phase != s.phase || s.done == s.total || at.elapsed() >= EMIT_EVERY
                }
            };
            if due {
                *last = Some((Instant::now(), s.phase));
                let _ = app.emit(PROGRESS_EVENT, s);
            }
        };
        let result = state
            .engine
            .index_pass(&folders, &extractors, options, &on_status);
        match &result {
            Ok(summary) => {
                tracing::info!(?summary, "index pass done");
                if options.retry_empty && summary.errors.is_empty() {
                    let _ = state.update_settings(|s| {
                        s.extractor_level = EXTRACTOR_LEVEL;
                        Ok(())
                    });
                }
            }
            Err(e) => tracing::error!("index pass failed: {e:#}"),
        }
        let _ = app.emit(DONE_EVENT, result.map_err(|e| format!("{e:#}")));
        if !state.engine.finish_pass() {
            break;
        }
    }
}
