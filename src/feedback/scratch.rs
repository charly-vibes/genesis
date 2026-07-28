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

use serde::{Deserialize, Serialize};
use std::io::Write;
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

/// Write an error scratch record.
///
/// Best-effort: returns `Ok(())` on success, `Err` on failure (but the caller
/// should generally ignore the error since this must never shadow the real error).
pub fn write_error_scratch(tool_name: &str, record: &ErrorRecord) -> std::io::Result<()> {
    let dir = scratch_dir(tool_name);
    let path = scratch_path(tool_name);

    // Ensure the directory exists
    std::fs::create_dir_all(&dir)?;

    // Serialize the record
    let line = serde_json::to_string(record)?;

    // Open in append mode, create if not exists
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;

    // Write the line
    writeln!(file, "{}", line)?;

    // Cap the file to MAX_ENTRIES
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

/// Read the last error scratch record for a tool.
///
/// Returns `None` if the file doesn't exist, is empty, or can't be parsed.
pub fn read_last_error(tool_name: &str) -> Option<ErrorRecord> {
    let path = scratch_path(tool_name);
    let content = std::fs::read_to_string(path).ok()?;
    let last_line = content.lines().last()?;
    serde_json::from_str(last_line).ok()
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

/// Cap the scratch file to MAX_ENTRIES lines.
///
/// Reads the file, keeps only the last MAX_ENTRIES lines, and writes back.
fn cap_scratch_file(path: &PathBuf) -> std::io::Result<()> {
    let content = std::fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();

    if lines.len() <= MAX_ENTRIES {
        return Ok(());
    }

    let trimmed = lines[lines.len() - MAX_ENTRIES..].join("\n");
    std::fs::write(path, trimmed + "\n")?;
    Ok(())
}

/// Generate an ISO 8601 timestamp for the current time.
#[cfg(test)]
fn timestamp() -> String {
    // Simple timestamp without external dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Format as ISO 8601 date-time (UTC)
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Compute year/month/day from days since epoch
    // Simple algorithm for dates after 2000
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 0usize;
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as i64 {
            m = i + 1;
            break;
        }
        remaining -= md as i64;
    }
    if m == 0 {
        m = 12;
    }
    let d = (remaining + 1) as u8;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

#[cfg(test)]
fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_record_serializes() {
        let record = ErrorRecord {
            ts: "2026-07-28T00:00:00Z".into(),
            argv: vec!["genesis".into(), "check".into()],
            exit: 1,
            footer: Some("→ Run: genesis doctor".into()),
            kind: "Fix".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(json.contains("2026-07-28T00:00:00Z"));
        assert!(json.contains("genesis"));
        assert!(json.contains("genesis doctor"));
        assert!(json.contains("\"kind\":\"Fix\""));
    }

    #[test]
    fn test_error_record_omits_footer_when_none() {
        let record = ErrorRecord {
            ts: "2026-07-28T00:00:00Z".into(),
            argv: vec!["genesis".into()],
            exit: 0,
            footer: None,
            kind: "Ok".into(),
        };
        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("footer"));
    }

    #[test]
    fn test_write_and_read_last_error() {
        let tool = "_test_genesis_scratch";
        let record = ErrorRecord {
            ts: "2026-07-28T00:00:00Z".into(),
            argv: vec!["test".into()],
            exit: 1,
            footer: None,
            kind: "Test".into(),
        };

        write_scratch_best_effort(tool, &record);
        let last = read_last_error(tool);
        assert!(last.is_some());
        assert_eq!(last.unwrap().exit, 1);

        // Cleanup
        let _ = std::fs::remove_file(scratch_path(tool));
        let _ = std::fs::remove_dir(scratch_dir(tool));
    }

    #[test]
    fn test_read_last_error_nonexistent() {
        let tool = "_test_nonexistent_tool";
        let last = read_last_error(tool);
        assert!(last.is_none());
    }

    #[test]
    fn test_read_all_errors() {
        let tool = "_test_genesis_all";
        for i in 0..3 {
            let record = ErrorRecord {
                ts: format!("2026-07-28T00:00:0{}Z", i),
                argv: vec!["test".into(), format!("{}", i)],
                exit: i,
                footer: None,
                kind: "Test".into(),
            };
            write_scratch_best_effort(tool, &record);
        }

        let all = read_all_errors(tool);
        assert_eq!(all.len(), 3);
        assert_eq!(all[0].exit, 0);
        assert_eq!(all[2].exit, 2);

        // Cleanup
        let _ = std::fs::remove_file(scratch_path(tool));
        let _ = std::fs::remove_dir(scratch_dir(tool));
    }

    #[test]
    fn test_cap_scratch_file() {
        let tool = "_test_genesis_cap";
        let path = scratch_path(tool);
        let dir = scratch_dir(tool);
        std::fs::create_dir_all(&dir).unwrap();

        // Write MAX_ENTRIES + 10 lines
        for i in 0..MAX_ENTRIES + 10 {
            let record = ErrorRecord {
                ts: format!("2026-07-28T00:00:0{}Z", i % 10),
                argv: vec!["test".into()],
                exit: i as i32,
                footer: None,
                kind: "Test".into(),
            };
            let line = serde_json::to_string(&record).unwrap();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap();
            writeln!(file, "{}", line).unwrap();
        }

        // Cap
        cap_scratch_file(&path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        assert_eq!(lines.len(), MAX_ENTRIES);

        // Cleanup
        let _ = std::fs::remove_file(scratch_path(tool));
        let _ = std::fs::remove_dir(scratch_dir(tool));
    }

    #[test]
    fn test_timestamp_format() {
        let ts = timestamp();
        // ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
        assert_eq!(ts.len(), 20, "expected ISO 8601 length, got: {}", ts);
        assert!(ts.ends_with('Z'), "expected UTC timezone marker");
        assert_eq!(&ts[4..5], "-", "expected dash after year");
        assert_eq!(&ts[7..8], "-", "expected dash after month");
        assert_eq!(&ts[10..11], "T", "expected T separator");
    }

    #[test]
    fn test_scratch_dir_uses_xdg() {
        unsafe { std::env::set_var("XDG_CACHE_HOME", "/custom/cache") };
        let dir = scratch_dir("test-tool");
        assert_eq!(dir, PathBuf::from("/custom/cache/test-tool"));
        unsafe { std::env::remove_var("XDG_CACHE_HOME") };
    }
}
