//! Integration tests for the JSON helpers (add-fs-module task 1.7).
//!
//! Contract: `atomic_write_json` serializes through the atomic replace path,
//! `read_json` round-trips the value, and malformed JSON yields a typed error —
//! never a panic.

mod common;

use common::TestUnwrap;

use genesis::fs::{atomic_write_json, read_json};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Config {
    name: String,
    retries: u32,
}

/// Scenario: a JSON file round-trips.
#[test]
fn json_round_trips() {
    let tmp = tempfile::tempdir().require("tempdir");
    let path = tmp.path().join("config.json");

    let written = Config {
        name: "genesis".to_string(),
        retries: 3,
    };
    atomic_write_json(&path, &written).require("write json");
    let read: Config = read_json(&path).require("read json");
    assert_eq!(
        read, written,
        "the value read back must equal the value written"
    );
}

/// Scenario: malformed JSON yields a typed error.
#[test]
fn malformed_json_yields_typed_error() {
    let tmp = tempfile::tempdir().require("tempdir");
    let path = tmp.path().join("broken.json");
    std::fs::write(&path, "{not json at all").require("seed malformed file");

    let err: genesis::fs::FsError = read_json::<Config>(&path).expect_err("must not parse");
    assert!(
        matches!(&err, genesis::fs::FsError::Json { path: p, .. } if p == &path),
        "malformed JSON must yield the typed JSON error, got {err:?}"
    );
    // And it must render as an error, not panic.
    assert!(!err.to_string().is_empty());
}

/// A missing file is a typed io error, distinct from the JSON error.
#[test]
fn missing_file_yields_typed_io_error() {
    let missing = std::env::temp_dir().join("genesis-fs-missing.json");
    let _ = std::fs::remove_file(&missing);
    let err: genesis::fs::FsError = read_json::<Config>(&missing).expect_err("must not exist");
    assert!(
        matches!(err, genesis::fs::FsError::Io(_)),
        "a missing file must yield the typed io error, got {err:?}"
    );
}

/// `atomic_write_json` replaces an existing file atomically: prior content is
/// fully replaced, and no staging debris remains.
#[test]
fn atomic_write_json_replaces_prior_content() {
    let tmp = tempfile::tempdir().require("tempdir");
    let path = tmp.path().join("config.json");
    atomic_write_json(
        &path,
        &Config {
            name: "old".to_string(),
            retries: 0,
        },
    )
    .require("seed");

    let updated = Config {
        name: "new".to_string(),
        retries: 9,
    };
    atomic_write_json(&path, &updated).require("rewrite");

    let read: Config = read_json(&path).require("read back");
    assert_eq!(read, updated);

    let siblings: Vec<_> = std::fs::read_dir(tmp.path())
        .require("read dir")
        .flatten()
        .map(|e: std::fs::DirEntry| e.path())
        .filter(|p: &std::path::PathBuf| p != &path)
        .collect();
    assert!(siblings.is_empty(), "no staging leftovers: {siblings:?}");
}
