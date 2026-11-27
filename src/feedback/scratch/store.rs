//! Scratch-file store: append error records and cap the file size.
//!
//! ## Responsibilities
//!
//! - Append one JSONL record per failure, creating the cache dir on demand
//! - Rotate the file down to `MAX_ENTRIES` lines
//!
//! ## Rationale
//!
//! Split from `mod.rs` so each step stays under the R8 profile
//! function-size limits; entry points are re-exported unchanged.

use std::io::Write;
use std::path::PathBuf;

use super::{ErrorRecord, MAX_ENTRIES, scratch_dir, scratch_path};

/// Write an error scratch record.
///
/// Best-effort: returns `Ok(())` on success, `Err` on failure (but the caller
/// should generally ignore the error since this must never shadow the real error).
pub fn write_error_scratch(tool_name: &str, record: &ErrorRecord) -> std::io::Result<()> {
    let path = scratch_path(tool_name);
    std::fs::create_dir_all(scratch_dir(tool_name))?;
    append_line(&path, &serde_json::to_string(record)?)?;
    cap_scratch_file(&path)?;
    Ok(())
}

/// Write an error scratch record, best-effort (silently ignores errors).
///
/// This is the recommended entry point for error sinks — it never shadows
/// the real error.
pub fn write_scratch_best_effort(tool_name: &str, record: &ErrorRecord) {
    let _ = write_error_scratch(tool_name, record);
}

/// Open `path` in create+append mode and write one line.
///
/// The formatted line is flushed with a single `write_all` so concurrent
/// readers see whole lines (or skip the torn tail — the reader documents
/// interleaved-line tolerance), never interleaved fragments.
fn append_line(path: &PathBuf, line: &str) -> std::io::Result<()> {
    let mut buf = line.as_bytes().to_vec();
    buf.push(b'\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    file.write_all(&buf)
}

/// Cap the scratch file to MAX_ENTRIES lines.
///
/// Atomic via stage-then-rename (genesis-4xl): the tail is written to a
/// sibling tmp file and `rename`d over the scratch file, so a concurrent
/// reader sees either the old or the new complete file — never an empty or
/// partial one. Once `genesis::fs::atomic_write` exists (genesis-mqt) this
/// should delegate to it.
///
/// Trade-off of the rename: appends that land between the snapshot read and
/// the rename are replaced by it and dropped; append-mode writers recover on
/// the next record. Capping is best-effort housekeeping, so that window is
/// accepted over the old truncate-to-write window that could lose *every*
/// concurrent record and present readers an empty file.
pub(super) fn cap_scratch_file(path: &PathBuf) -> std::io::Result<()> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= MAX_ENTRIES {
        return Ok(());
    }

    let trimmed = lines[lines.len() - MAX_ENTRIES..].join("\n");
    let tmp = path.with_extension("jsonl.tmp");
    std::fs::write(&tmp, trimmed + "\n")?;
    if let Err(err) = std::fs::rename(&tmp, path) {
        let _ = std::fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}
