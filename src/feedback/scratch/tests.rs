//! Unit tests for `feedback::scratch`: record serialization, write/read
//! round-trips, file capping, and cache-path resolution.

use std::io::Write;
use std::path::Path;

use super::store::cap_scratch_file;
use super::{
    ErrorRecord, MAX_ENTRIES, read_last_error, scratch_dir, scratch_path, write_scratch_best_effort,
};
use crate::testutil::TestUnwrap;

/// Build a minimal `ErrorRecord` for tests.
fn record(ts: &str, exit: i32, kind: &str, argv: Vec<String>) -> ErrorRecord {
    ErrorRecord {
        ts: ts.into(),
        argv,
        exit,
        footer: None,
        kind: kind.into(),
    }
}

/// Serialize `record` to JSON and append it to `path` as one JSONL line.
fn append_record(path: &Path, record: &ErrorRecord) {
    let line = serde_json::to_string(record).require("serialize record");
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .require("open errors file");
    writeln!(file, "{}", line).require("append record");
}

/// Append a `["test"]`-argv record with the given timestamp and exit code.
fn append_simple_record(path: &Path, ts: &str, exit: i32) {
    append_record(path, &record(ts, exit, "Test", vec!["test".into()]));
}

/// Append `n` sequential test records as JSONL lines to `path`.
fn append_test_records(path: &Path, n: usize) {
    for i in 0..n {
        let rec = record(
            &format!("2026-07-28T00:00:0{}Z", i),
            i as i32,
            "Test",
            vec!["test".into(), i.to_string()],
        );
        append_record(path, &rec);
    }
}

/// Remove the scratch file and dir for `tool` (test setup/teardown).
fn cleanup_scratch(tool: &str) {
    let _ = std::fs::remove_file(scratch_path(tool));
    let _ = std::fs::remove_dir(scratch_dir(tool));
}

#[test]
fn test_error_record_serializes() {
    let mut record = record(
        "2026-07-28T00:00:00Z",
        1,
        "Fix",
        vec!["genesis".into(), "check".into()],
    );
    record.footer = Some("→ Run: genesis doctor".into());
    let json = serde_json::to_string(&record).require("value");
    assert!(json.contains("2026-07-28T00:00:00Z"));
    assert!(json.contains("genesis"));
    assert!(json.contains("genesis doctor"));
    assert!(json.contains("\"kind\":\"Fix\""));
}

#[test]
fn test_error_record_omits_footer_when_none() {
    let record = record("2026-07-28T00:00:00Z", 0, "Ok", vec!["genesis".into()]);
    let json = serde_json::to_string(&record).require("value");
    assert!(!json.contains("footer"));
}

/// Uses a per-pid tool name so parallel test runs cannot race on the scratch file.
#[test]
fn test_write_and_read_last_error() {
    let tool = format!("_test_gs_{}", std::process::id());
    cleanup_scratch(&tool);

    let record = record("2026-07-28T00:00:00Z", 1, "Test", vec!["test".into()]);
    write_scratch_best_effort(&tool, &record);
    let last = read_last_error(&tool);
    assert!(last.is_some());
    assert_eq!(last.require("value").exit, 1);

    // Cleanup
    cleanup_scratch(&tool);
}

#[test]
fn test_read_last_error_nonexistent() {
    let tool = "_test_nonexistent_tool";
    let last = read_last_error(tool);
    assert!(last.is_none());
}

#[test]
fn test_read_all_errors() {
    let dir = tempfile::tempdir().require("tempdir");
    let path = dir.path().join("errors.jsonl");
    append_test_records(&path, 3);

    let content = std::fs::read_to_string(&path).require("value");
    let all: Vec<ErrorRecord> = content
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert_eq!(all.len(), 3);
    assert_eq!(all[0].exit, 0);
    assert_eq!(all[2].exit, 2);
    // dir is cleaned up on drop
}

#[test]
fn test_cap_scratch_file() {
    let dir = tempfile::tempdir().require("tempdir");
    let path = dir.path().join("errors.jsonl");

    // Write MAX_ENTRIES + 10 lines
    for i in 0..MAX_ENTRIES + 10 {
        append_simple_record(&path, &format!("2026-07-28T00:00:0{}Z", i % 10), i as i32);
    }
    // Cap
    cap_scratch_file(&path).require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(content.lines().count(), MAX_ENTRIES);
    // dir is cleaned up on drop
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
    let prev = std::env::var("XDG_CACHE_HOME").ok();
    unsafe { std::env::set_var("XDG_CACHE_HOME", "/custom/cache") };
    let dir = scratch_dir("test-tool");
    assert_eq!(dir, std::path::PathBuf::from("/custom/cache/test-tool"));
    match prev {
        Some(v) => unsafe { std::env::set_var("XDG_CACHE_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CACHE_HOME") },
    }
}

/// Generate an ISO 8601 timestamp for the current time.
fn timestamp() -> String {
    // Simple timestamp without external dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    // Format as ISO 8601 date-time (UTC)
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let (y, m, d) = civil_from_days_since_epoch(secs / 86400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

/// Compute `(year, month, day)` from whole days since 1970-01-01.
/// Simple algorithm, exact for dates after 2000.
fn civil_from_days_since_epoch(days: u64) -> (i64, usize, u8) {
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
    let m = month_for_remaining(remaining as u64, is_leap(y));
    let d = (remaining + 1) as u8;
    (y, m, d)
}

/// 1-based month number given days remaining within the year.
fn month_for_remaining(mut remaining: u64, leap: bool) -> usize {
    let month_days = if leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    for (i, &md) in month_days.iter().enumerate() {
        if remaining < md as u64 {
            return i + 1;
        }
        remaining -= md as u64;
    }
    12
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}
