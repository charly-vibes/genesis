//! Error-scratch JSONL writer (§4 of agent-issue-reporting.md).
//!
//! Persists the last error on non-zero exit for `--from-last-error`.
//!
//! ## Design
//!
//! - Writes one-line JSONL to `$XDG_CACHE_HOME/<tool>/errors.jsonl`
//! - Rotated/capped to last 100 entries
//! - Best-effort: never shadows the real error
//! - Fallback to `std::env::temp_dir()` if cache dir unavailable
//! - Append-mode with line-atomic writes
//!
//! ## Responsibilities
//!
//! - Public record type (`ErrorRecord`) and cache-path resolution
//! - Re-export the `store`/`reader` entry points (public API unchanged)

mod reader;
mod store;
#[cfg(test)]
mod tests;

pub use reader::{read_all_errors, read_last_error};
pub use store::{write_error_scratch, write_scratch_best_effort};

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default maximum number of error scratch entries.
const MAX_ENTRIES: usize = 100;

/// An error scratch record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// ISO 8601 timestamp.
    pub ts: String,
    /// The command that failed.
    pub argv: Vec<String>,
    /// Exit code.
    pub exit: i32,
    /// Suggestion footer, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    /// Kind of error.
    pub kind: String,
}

/// Get the error scratch directory for a tool.
///
/// Uses `$XDG_CACHE_HOME/<tool>` if set, otherwise `~/.cache/<tool>`.
/// Falls back to `std::env::temp_dir()` if neither is available.
fn scratch_dir(tool_name: &str) -> PathBuf {
    let base = if let Ok(cache) = std::env::var("XDG_CACHE_HOME") {
        PathBuf::from(cache)
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".cache")
    } else {
        std::env::temp_dir()
    };
    base.join(tool_name)
}

/// Get the path to the error scratch file for a tool.
fn scratch_path(tool_name: &str) -> PathBuf {
    scratch_dir(tool_name).join("errors.jsonl")
}
