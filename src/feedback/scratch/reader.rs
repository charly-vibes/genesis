//! Scratch-file readers.
//!
//! ## Responsibilities
//!
//! - Read the last parseable `ErrorRecord` (reverse scan skips junk lines)
//! - Read every parseable record, oldest first
//!
//! ## Rationale
//!
//! Split from `mod.rs` to keep writer and reader concerns separate; entry
//! points are re-exported unchanged.

use super::{ErrorRecord, scratch_path};

/// Read the last error scratch record for a tool.
///
/// Returns `None` if the file doesn't exist, is empty, or can't be parsed.
/// If concurrent writes caused interleaved lines, malformed lines are
/// silently skipped (the last *parseable* line is returned).
pub fn read_last_error(tool_name: &str) -> Option<ErrorRecord> {
    let path = scratch_path(tool_name);
    let content = std::fs::read_to_string(path).ok()?;
    // Iterate in reverse to find the last parseable line
    content
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<ErrorRecord>(line).ok())
}

/// Read all error scratch records for a tool (newest last).
pub fn read_all_errors(tool_name: &str) -> Vec<ErrorRecord> {
    let path = scratch_path(tool_name);
    let content = std::fs::read_to_string(&path).unwrap_or_default();
    content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}
