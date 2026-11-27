//! Unit tests for `feedback::scratch`: record serialization, write/read
//! round-trips, file capping, and cache-path resolution.

use std::io::Write;
use std::path::Path;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use super::store::cap_scratch_file;
use super::{
    ErrorRecord, MAX_ENTRIES, read_last_error, scratch_dir, scratch_path, write_scratch_best_effort,
};
use crate::testutil::TestUnwrap;

/// Build a minimal `ErrorRecord` for tests (via `ErrorRecord::new`).
fn record(ts: &str, exit: i32, kind: &str, argv: Vec<String>) -> ErrorRecord {
    ErrorRecord::new(ts, argv, exit, kind)
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

/// Legacy records written before the `message` field existed must
/// deserialize (serde default), yielding `message: None`.
#[test]
fn test_legacy_record_without_message_deserializes() {
    let tool = "test-legacy-c9x";
    cleanup_scratch(tool);
    let path = scratch_path(tool);
    std::fs::create_dir_all(scratch_dir(tool)).require("create scratch dir");
    std::fs::write(
        &path,
        r#"{"ts":"2026-07-29T12:00:00Z","argv":["test","check"],"exit":1,"footer":"run doctor","kind":"error"}"#,
    )
    .require("write legacy jsonl");
    let record = read_last_error(tool).require("legacy record should parse");
    assert_eq!(record.exit, 1);
    assert_eq!(record.message, None);
    cleanup_scratch(tool);
}

/// genesis-xxx: `ErrorRecord::new` defaults the optional fields, and
/// `..Default::default()` literals let field lists lag new fields.
#[test]
fn test_error_record_new_and_default() {
    let r = ErrorRecord::new("2026-07-28T00:00:00Z", vec!["genesis".into()], 1, "Fix");
    assert_eq!(r.ts, "2026-07-28T00:00:00Z");
    assert_eq!(r.argv, vec!["genesis".to_string()]);
    assert_eq!(r.exit, 1);
    assert_eq!(r.kind, "Fix");
    assert_eq!(r.footer, None);
    assert_eq!(r.message, None);
    assert_default_literal_shape();
}

/// Partial literal closed by Default (the documented construction shape),
/// and a Default-built record still round-trips through JSONL.
fn assert_default_literal_shape() {
    let partial = ErrorRecord {
        ts: "t".into(),
        argv: vec![],
        exit: 2,
        kind: "bug".into(),
        ..ErrorRecord::default()
    };
    assert_eq!(partial.footer, None);
    assert_eq!(partial.message, None);
    let json = serde_json::to_string(&partial).require("serialize");
    let back: ErrorRecord = serde_json::from_str(&json).require("deserialize");
    assert_eq!(back.ts, "t");
    assert_eq!(back.exit, 2);
    assert_eq!(back.kind, "bug");
    assert_eq!((back.footer, back.message), (None, None));
}

/// Uses a per-pid tool name so parallel test runs cannot race on the scratch file.
#[test]
fn test_write_and_read_last_error() {
    let tool = format!("_test_gs_{}", std::process::id());
    cleanup_scratch(&tool);
    let rec = record("2026-07-28T00:00:00Z", 1, "Test", vec!["test".into()]);
    write_scratch_best_effort(&tool, &rec);
    let last = read_last_error(&tool);
    assert!(last.is_some());
    assert_eq!(last.require("value").exit, 1);
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
}

#[test]
fn test_cap_scratch_file() {
    let dir = tempfile::tempdir().require("tempdir");
    let path = dir.path().join("errors.jsonl");

    for i in 0..MAX_ENTRIES + 10 {
        append_simple_record(&path, &format!("2026-07-28T00:00:0{}Z", i % 10), i as i32);
    }
    cap_scratch_file(&path).require("value");
    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(content.lines().count(), MAX_ENTRIES);
}

/// genesis-4xl: cap must stay correct while another writer appends — the
/// reader never observes an empty or partial file, every surviving line
/// parses, the file stays bounded, and no tmp staging file lingers.
#[test]
fn test_cap_scratch_file_concurrent_append() {
    let dir = tempfile::tempdir().require("tempdir");
    let path = dir.path().join("errors.jsonl");

    // Seed over the cap so every cap call performs a stage+rename rewrite.
    append_test_records(&path, MAX_ENTRIES + 5);

    let done = Arc::new(AtomicBool::new(false));
    let writer = spawn_concurrent_writer(&path, done.clone());
    while !done.load(Ordering::Acquire) {
        cap_scratch_file(&path).require("cap under concurrent append");
        assert_reader_never_saw_empty_partial(&path);
    }
    writer.join().expect("writer thread");
    assert_final_scratch_state(dir.path(), &path);
}

/// Append 300 records on a side thread, flipping `done` (Release) when finished.
fn spawn_concurrent_writer(path: &Path, done: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let writer_path = path.to_owned();
    std::thread::spawn(move || {
        for i in 0..300u32 {
            let ts = format!("2026-07-28T00:00:{:02}Z", i % 60);
            let rec = record(
                &ts,
                i as i32,
                "Concurrent",
                vec!["test".into(), i.to_string()],
            );
            append_record(&writer_path, &rec);
        }
        done.store(true, Ordering::Release);
    })
}

/// At least one line parses as an `ErrorRecord`.
fn any_record_parses(content: &str) -> bool {
    content
        .lines()
        .any(|l| serde_json::from_str::<ErrorRecord>(l).is_ok())
}

/// A non-empty file must always carry at least one parseable record (never empty-partial).
fn assert_reader_never_saw_empty_partial(path: &Path) {
    if let Ok(content) = std::fs::read_to_string(path) {
        assert!(
            any_record_parses(&content),
            "reader saw empty/partial content during cap: {content:?}"
        );
    }
}

/// Final state: bounded, parseable, no tmp staging siblings.
fn assert_final_scratch_state(dir_path: &Path, path: &std::path::PathBuf) {
    cap_scratch_file(path).require("final cap");
    let content = std::fs::read_to_string(path).require("value");
    assert!(
        any_record_parses(&content),
        "final file must keep at least one record"
    );
    let lines = content.lines().count();
    assert!(
        lines <= MAX_ENTRIES,
        "cap must keep the file bounded, got {lines} lines"
    );
    assert_no_staging_siblings(dir_path, path);
}

/// No tmp staging file may linger next to the scratch file.
fn assert_no_staging_siblings(dir_path: &Path, path: &Path) {
    let siblings: Vec<_> = std::fs::read_dir(dir_path)
        .require("read dir")
        .flatten()
        .map(|e| e.path())
        .filter(|p| *p != path)
        .collect();
    assert!(
        siblings.is_empty(),
        "tmp staging files must not linger: {siblings:?}"
    );
}

#[test]
fn test_error_sink_record_carries_message() {
    let tool = format!("_test_sink_{}", std::process::id());
    cleanup_scratch(&tool);
    run_error_sink(&tool);
    assert_sink_record_carries_message(&tool);
    cleanup_scratch(&tool);
}

/// Feed one io::Error through the genesis-owned capture site.
fn run_error_sink(tool: &str) {
    let sink = crate::guide::ErrorSink::new(tool);
    let err = std::io::Error::other("something broke badly");
    let mut stderr = Vec::new();
    sink.handle(&err, &mut stderr);
}

/// genesis-xxx: the genesis-owned capture site (`guide::ErrorSink`) must
/// persist a scratch record carrying the rendered error message.
fn assert_sink_record_carries_message(tool: &str) {
    let record = read_last_error(tool).require("scratch record");
    assert_eq!(record.message.as_deref(), Some("something broke badly"));
    assert_eq!(record.kind, "error");
    assert_eq!(record.exit, 1);
    assert_eq!(record.argv, vec![tool.to_string()]);
    assert!(
        record.ts.ends_with('Z'),
        "ts must be ISO 8601: {}",
        record.ts
    );
}

mod paths;
