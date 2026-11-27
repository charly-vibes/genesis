//! Integration tests for `genesis::fs::atomic_write` (add-fs-module task 1.1).
//!
//! Contract: the target is replaced in one observable step; a failure before
//! the rename leaves the prior target byte-for-byte intact and removes the
//! staging file; the staging file is a sibling of the target, never a system
//! temp file.

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

mod common;

use common::TestUnwrap;
use genesis::fs::{atomic_write, staging_path_for};

/// Scenario: a reader never observes a partial file.
///
/// While the writer replaces the target in a loop, a concurrent reader must
/// always observe one of the two complete payloads — never a truncated or
/// interleaved file.
#[test]
fn reader_never_observes_partial_file() {
    let tmp = tempfile::tempdir().require("tempdir");
    let path = tmp.path().join("data.bin");

    const SHORT: usize = 4096;
    const LONG: usize = 8192;
    atomic_write(&path, &[b'a'; SHORT]).require("initial write");

    let stop = Arc::new(AtomicBool::new(false));
    let stop_reader = Arc::clone(&stop);
    let read_path = path.clone();
    let reader = std::thread::spawn(move || {
        while !stop_reader.load(Ordering::Relaxed) {
            if let Ok(content) = std::fs::read(&read_path) {
                assert!(
                    content == vec![b'a'; SHORT] || content == vec![b'b'; LONG],
                    "reader observed a partial file: {} bytes",
                    content.len()
                );
            }
        }
    });

    for _ in 0..24 {
        atomic_write(&path, &[b'b'; LONG]).require("write long");
        atomic_write(&path, &[b'a'; SHORT]).require("write short");
    }

    stop.store(true, Ordering::Relaxed);
    reader.join().require("reader thread");

    // The last write was the short payload; the final file state must match it.
    assert_eq!(
        std::fs::read(&path).require("read final state"),
        vec![b'a'; SHORT],
        "the target must hold the final payload after the writes finish"
    );
}

/// Scenario: the prior content survives a failed write.
///
/// `atomic_write` to a path that cannot be renamed over (the target exists as
/// a directory) must report the error, leave the target untouched, and remove
/// the staging file.
#[test]
fn failed_write_leaves_target_unchanged() {
    // Case 1: the "target" is a directory, so the rename must fail. The
    // staging file is created in the target's parent directory; the failure
    // must not leave it behind.
    let tmp = tempfile::tempdir().require("tempdir");
    let target_dir = tmp.path().join("target");
    std::fs::create_dir_all(&target_dir).require("create target dir");
    let err = atomic_write(&target_dir, b"payload").expect_err("rename must fail");
    assert!(
        matches!(err, genesis::fs::FsError::Io(_)),
        "expected an io error, got {err:?}"
    );
    let siblings: Vec<_> = std::fs::read_dir(tmp.path())
        .require("read dir")
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(
        siblings,
        vec![target_dir.clone()],
        "staging file must be removed after a failed rename: {siblings:?}"
    );

    #[cfg(unix)]
    failed_open_in_readonly_dir_keeps_target();
}

/// Case 2 (Unix): a read-only target directory. The staging open must fail in
/// that directory — proving the staging file was attempted there, not in a
/// system temp dir — and the prior target must stay byte-for-byte intact.
#[cfg(unix)]
fn failed_open_in_readonly_dir_keeps_target() {
    let dir = tempfile::tempdir().require("tempdir");
    let path = dir.path().join("kept.txt");
    std::fs::write(&path, b"prior bytes").require("seed target");

    let original = make_dir_readonly(dir.path()).require("chmod dir read-only");

    let err = atomic_write(&path, b"new bytes").expect_err("staging open must fail");
    let io = match err {
        genesis::fs::FsError::Io(io) => io,
        other => panic!("expected an io error, got {other:?}"),
    };
    // The failure is the staging open being refused by the read-only
    // target directory itself.
    assert_eq!(
        io.raw_os_error(),
        Some(libc_const_eacces()),
        "staging must be attempted inside the target's own directory: {io}"
    );

    restore_dir_permissions(dir.path(), original).require("chmod dir back");

    assert_no_staging_siblings(dir.path(), &path, "no staging file may remain");
    assert_eq!(
        std::fs::read(&path).require("read kept target"),
        b"prior bytes",
        "failed write must leave the target byte-for-byte unchanged"
    );
}

/// Set the directory mode to `0o500` and return the original mode.
#[cfg(unix)]
fn make_dir_readonly(dir: &Path) -> std::io::Result<u32> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = std::fs::metadata(dir)?.permissions();
    let original = perms.mode();
    perms.set_mode(0o500);
    std::fs::set_permissions(dir, perms)?;
    Ok(original)
}

/// Restore a directory's mode after [`make_dir_readonly`], so the tempdir can
/// be removed.
#[cfg(unix)]
fn restore_dir_permissions(dir: &Path, original: u32) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = std::fs::metadata(dir)?.permissions();
    perms.set_mode(original);
    std::fs::set_permissions(dir, perms)
}

/// Assert the directory contains only `keep` — no staging debris.
#[cfg(unix)]
fn assert_no_staging_siblings(dir: &Path, keep: &Path, message: &str) {
    let siblings: Vec<_> = std::fs::read_dir(dir)
        .require("read dir")
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(
        siblings,
        vec![keep.to_path_buf()],
        "{message}: {siblings:?}"
    );
}

/// `EACCES` on every Unix flavor this crate supports.
#[cfg(unix)]
fn libc_const_eacces() -> i32 {
    std::io::Error::from_raw_os_error(13)
        .raw_os_error()
        .require("EACCES")
}

/// Scenario: the rename happens on the same filesystem.
///
/// The staging path is a sibling of the target (same parent directory), so the
/// rename never crosses a filesystem boundary.
#[test]
fn staging_file_lives_in_target_directory() {
    let tmp = tempfile::tempdir().require("tempdir");
    let path = tmp.path().join("sibling.bin");

    let staging = staging_path_for(&path);
    assert_eq!(
        staging.parent(),
        path.parent(),
        "staging file must live in the target's own directory"
    );
    let system_tmp = std::env::temp_dir();
    assert_ne!(
        staging.parent(),
        Some(system_tmp.as_path()),
        "staging file must not be a system temp dir"
    );

    // And end to end: a successful write leaves no staging debris anywhere
    // next to the target.
    atomic_write(&path, b"payload").require("write");
    let siblings: Vec<_> = std::fs::read_dir(tmp.path())
        .require("read dir")
        .flatten()
        .map(|e| e.path())
        .collect();
    assert_eq!(
        siblings,
        vec![path.clone()],
        "no staging leftovers: {siblings:?}"
    );
}
