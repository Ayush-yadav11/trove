//! User settings persisted as JSON in the app config dir.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const DEFAULT_SHORTCUT: &str = "Alt+Shift+Space";

/// Bump when the indexer learns to read something it couldn't before, so
/// files recorded as "no text" get one more look. 1 = OCR.
pub const EXTRACTOR_LEVEL: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Folders indexed and kept in sync, in the order the user added them.
    pub folders: Vec<PathBuf>,
    /// Global shortcut that toggles the search window.
    pub shortcut: String,
    /// [`EXTRACTOR_LEVEL`] of the last completed index pass.
    pub extractor_level: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            folders: Vec::new(),
            shortcut: DEFAULT_SHORTCUT.into(),
            extractor_level: 0,
        }
    }
}

impl Settings {
    /// Missing or unreadable settings fall back to defaults rather than
    /// blocking startup; a corrupt file is kept aside for inspection.
    pub fn load(path: &Path) -> Self {
        let Ok(raw) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("settings unreadable ({e}); starting fresh");
                let _ = std::fs::rename(path, path.with_extension("json.bad"));
                Self::default()
            }
        }
    }

    /// Write atomically (temp file + rename) so a crash never truncates it.
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
        }
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)
            .with_context(|| format!("writing {}", tmp.display()))?;
        std::fs::rename(&tmp, path).context("saving settings")?;
        Ok(())
    }
}
