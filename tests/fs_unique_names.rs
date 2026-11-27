//! Integration tests for `genesis::fs` staging-name generation (add-fs-module
//! task 1.3).
//!
//! Contract: staging names are unique per writer (pid + process-shared atomic
//! counter), so two concurrent writers to the same target never share a
//! staging path, repeated writes never reuse a name, and a successful write
//! leaves no staging file in the target directory.

use std::collections::HashSet;
use std::sync::Arc;

mod common;

use common::TestUnwrap;
use genesis::fs::{atomic_write, staging_path_for};

/// Scenario: concurrent writers use distinct staging files.
#[test]
fn concurrent_writers_use_distinct_staging_names() {
    let tmp = tempfile::tempdir().require("tempdir");
    let target = tmp.path().join("shared.json");

    let names: Arc<std::sync::Mutex<Vec<String>>> = Arc::default();
    let mut handles = Vec::new();
    for i in 0..4i32 {
        let target = target.clone();
        let names = Arc::clone(&names);
        handles.push(std::thread::spawn(move || {
            record_staging_name_and_write(&target, &names, i);
        }));
    }
    join_all(handles);

    let names: HashSet<_> = take_names(&names);
    assert_eq!(
        names.len(),
        4,
        "concurrent writers must use distinct staging names: {names:?}"
    );

    // Every staging name belongs to this target.
    for name in names {
        assert!(
            name.contains("shared.json"),
            "staging name must identify the target: {name}"
        );
    }
}

/// Join every writer thread, propagating any panic.
fn join_all(handles: Vec<std::thread::JoinHandle<()>>) {
    for handle in handles {
        handle.join().require("writer thread");
    }
}

/// Drain the recorded staging names into a set.
fn take_names(names: &Arc<std::sync::Mutex<Vec<String>>>) -> HashSet<String> {
    std::mem::take(&mut *names.lock().require("lock"))
        .into_iter()
        .collect()
}

/// Record the current staging name for `target` and perform a write through
/// it, so concurrent writers race on both the name generator and the rename.
fn record_staging_name_and_write(
    target: &std::path::Path,
    names: &std::sync::Mutex<Vec<String>>,
    writer: i32,
) {
    let staging = staging_path_for(target);
    names.lock().require("lock").push(
        staging
            .file_name()
            .require("name")
            .to_string_lossy()
            .into_owned(),
    );
    atomic_write(target, format!("writer {writer}").as_bytes()).require("concurrent write");
}

/// Scenario: repeated writes within one process do not collide.
#[test]
fn sequential_writes_do_not_reuse_staging_name() {
    let tmp = tempfile::tempdir().require("tempdir");
    let target = tmp.path().join("seq.json");

    let first = staging_path_for(&target);
    let second = staging_path_for(&target);
    assert_ne!(
        first, second,
        "sequential staging calls must not reuse a name"
    );

    atomic_write(&target, b"one").require("write one");
    atomic_write(&target, b"two").require("write two");
    assert_eq!(
        std::fs::read(&target).require("read target"),
        b"two",
        "the second sequential write must win"
    );
}

/// Scenario: a stale staging file is identifiable.
///
/// A staging file's name must be recognizable as staging debris of the target
/// (leading dot, `.tmp` suffix, pid embedded) so a later cleanup sweep can find
/// it — without any sweep being required for a successful write (asserted by
/// the no-leftover check in `staging_file_lives_in_target_directory`).
#[test]
fn staging_name_is_identifiable_as_staging_file() {
    let tmp = tempfile::tempdir().require("tempdir");
    let target = tmp.path().join("ident.json");

    let staging = staging_path_for(&target);
    let name = staging.file_name().require("name").to_string_lossy();
    assert!(
        name.starts_with('.'),
        "staging name must be dot-prefixed: {name}"
    );
    assert!(
        name.ends_with(".tmp"),
        "staging name must end in .tmp: {name}"
    );
    assert!(
        name.contains(std::process::id().to_string().as_str()),
        "staging name must embed the writing process id: {name}"
    );
    assert!(
        name.contains("ident.json"),
        "staging name must identify the target: {name}"
    );
}
