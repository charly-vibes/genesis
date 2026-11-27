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
///
/// Construct with [`ErrorRecord::new`] (then assign the optional fields), or
/// with a struct literal closed by `..Default::default()` — never an
/// exhaustive field list. New fields land with `#[serde(default)]` (legacy
/// JSONL keeps deserializing) and would otherwise break every exhaustive
/// literal in downstream consumers (genesis-xxx).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    /// The captured error message (miette text or stderr tail), if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ErrorRecord {
    /// Build a record from the required fields; `footer` and `message`
    /// start as `None` — assign them afterwards when a value is available.
    pub fn new(
        ts: impl Into<String>,
        argv: Vec<String>,
        exit: i32,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            ts: ts.into(),
            argv,
            exit,
            footer: None,
            kind: kind.into(),
            message: None,
        }
    }
}

/// Current UTC time as ISO 8601 (`YYYY-MM-DDTHH:MM:SSZ`), no external
/// dependencies. Best-effort: falls back to the epoch on clock failure.
///
/// The canonical timestamp for scratch records — consumers (wai, testaruda,
/// pretender) must use this instead of hand-rolling civil-date math or
/// writing raw epoch seconds (genesis-4xl).
pub fn now_iso() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let time_secs = secs % 86400;
    let (y, m, d) = civil_date(secs / 86400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        time_secs / 3600,
        (time_secs % 3600) / 60,
        time_secs % 60
    )
}

/// `(year, month, day)` from whole days since 1970-01-01 (exact for
/// post-2000 dates; simple algorithm, no external dependency).
fn civil_date(days: u64) -> (i64, u32, u32) {
    let (y, remaining) = civil_year(days as i64);
    let (m, d) = civil_month(y, remaining);
    (y, m, d)
}

/// Advance past whole years from 1970 until `days` fits inside year `y`;
/// returns `(y, days left inside y)`.
fn civil_year(days: i64) -> (i64, i64) {
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    (y, remaining)
}

/// `(month, day)` for `remaining` whole days since January 1 of year `y`.
fn civil_month(y: i64, remaining: i64) -> (u32, u32) {
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u32;
    let mut rem = remaining;
    for &md in &month_days {
        if rem < md {
            break;
        }
        rem -= md;
        m += 1;
    }
    (m, (rem + 1) as u32)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

/// Get the error scratch directory for a tool.
///
/// Uses `$XDG_CACHE_HOME/<tool>` if set, otherwise `~/.cache/<tool>`.
/// Falls back to `std::env::temp_dir()` if neither is available.
/// Empty values fall through (genesis-4xl): an empty `XDG_CACHE_HOME` must
/// never resolve to a relative path that writes `errors.jsonl` into the cwd.
fn scratch_dir(tool_name: &str) -> PathBuf {
    cache_base().join(tool_name)
}

/// Cache root: `$XDG_CACHE_HOME` if set and non-empty, else `$HOME/.cache`,
/// else the temp dir (same `.filter(|v| !v.is_empty())` treatment as
/// `update_check::cache_path`, genesis-4xl). Split from the env-reading
/// wrapper so tests can cover empty values without mutating process env.
fn cache_base() -> PathBuf {
    cache_base_for(
        std::env::var("XDG_CACHE_HOME").ok(),
        std::env::var("HOME").ok(),
    )
}

/// `cache_base` with injected env values.
fn cache_base_for(xdg: Option<String>, home: Option<String>) -> PathBuf {
    if let Some(cache) = xdg.filter(|v| !v.is_empty()) {
        return PathBuf::from(cache);
    }
    if let Some(home) = home.filter(|v| !v.is_empty()) {
        return PathBuf::from(home).join(".cache");
    }
    std::env::temp_dir()
}

/// Get the path to the error scratch file for a tool.
fn scratch_path(tool_name: &str) -> PathBuf {
    scratch_dir(tool_name).join("errors.jsonl")
}
