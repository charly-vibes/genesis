//! Contract tests for `genesis::git` tracked, content-hash, and
//! check-ignore queries.
//!
//! Spec: tracked reports files known to git independent of worktree
//! state; content hashes are deterministic (`hash-object` /
//! `rev-parse HEAD:<path>`); `is_ignored` is a tri-state single-path
//! mapping of `check-ignore -q` exit codes 0/1/≥128.

mod common;

use common::TestUnwrap;
use std::process::Command;

use genesis::fixture::Fixture;
use genesis::git::{self, GitError};

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_GIT_TEST_CHILD";

#[test]
fn tracked_lists_files_known_to_git() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("committed.txt"), "content").require("write file");
    for args in [
        &(["git", "add", "committed.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }
    std::fs::write(fixture.path("committed.txt"), "modified").require("modify tracked file");
    std::fs::write(fixture.path("never-added.txt"), "content").require("write untracked file");

    assert!(
        git::tracked(fixture.root(), &fixture.path("committed.txt")).require("tracked"),
        "a committed path must report tracked"
    );
    assert!(
        !git::tracked(fixture.root(), &fixture.path("never-added.txt")).require("tracked"),
        "a never-added path must report untracked"
    );
    // Tracked-ness is independent of worktree modifications: the modified
    // file above is still tracked.
    assert!(git::tracked(fixture.root(), &fixture.path("committed.txt")).require("tracked"));
}

#[test]
fn tracked_whole_repo_root_reports_tracked() {
    // path == root relativizes to "" which `ls-files --error-unmatch`
    // rejects with exit 128; the root itself must mean "the whole repo".
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("committed.txt"), "content").require("write file");
    for args in [
        &(["git", "add", "committed.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    assert!(
        git::tracked(fixture.root(), fixture.root()).require("tracked root"),
        "the repository root in a committed repo must report tracked"
    );
}

#[test]
fn tracked_glob_special_filename_matches_itself() {
    // Pathspec globs would make `a[1].txt` match `a1.txt`; callers pass
    // literal filenames, so the name must match itself only.
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("a[1].txt"), "bracketed").require("write bracketed file");
    std::fs::write(fixture.path("a1.txt"), "plain").require("write plain file");
    for args in [
        &(["git", "add", "."])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    assert!(
        git::tracked(fixture.root(), &fixture.path("a[1].txt")).require("tracked"),
        "a tracked file named a[1].txt must report tracked"
    );
    assert!(
        !git::is_ignored(fixture.root(), &fixture.path("a[1].txt")).require("is_ignored"),
        "a non-ignored file named a[1].txt must report not-ignored"
    );
}

#[test]
fn content_hash_is_deterministic() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("blob.txt"), "deterministic content\n").require("write file");

    let first =
        git::content_hash(fixture.root(), &fixture.path("blob.txt")).require("content_hash");
    let second =
        git::content_hash(fixture.root(), &fixture.path("blob.txt")).require("content_hash");
    assert_eq!(
        first, second,
        "content_hash must be deterministic per content"
    );

    // And it must equal git's own hash-object output for that file.
    let out = fixture
        .run(&["git", "hash-object", "blob.txt"])
        .require("run git hash-object");
    assert!(out.success(), "git hash-object failed: {}", out.stderr);
    assert_eq!(
        first,
        out.stdout.trim(),
        "hash must equal git hash-object output"
    );
}

#[test]
fn committed_content_hash_matches_head_blob() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("blob.txt"), "committed content\n").require("write file");
    for args in [
        &(["git", "add", "blob.txt"])[..],
        &(["git", "commit", "-m", "seed"])[..],
    ] {
        let out = fixture.run(args).require("run git");
        assert!(out.success(), "{args:?} failed: {}", out.stderr);
    }

    let committed =
        git::committed_content_hash(fixture.root(), &fixture.path("blob.txt")).require("hash");
    let worktree = git::content_hash(fixture.root(), &fixture.path("blob.txt")).require("hash");
    assert_eq!(
        committed, worktree,
        "committed hash must resolve HEAD:<path> and match the worktree blob"
    );
    assert_eq!(committed.len(), 40, "blob hash must be a full SHA-1");
}

#[test]
fn is_ignored_not_ignored_distinct_from_error() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitignore"), "ignored.txt\n").require("write gitignore");

    assert!(
        git::is_ignored(fixture.root(), &fixture.path("ignored.txt")).require("ignored"),
        "exit 0 must map to ignored"
    );
    assert!(
        !git::is_ignored(fixture.root(), &fixture.path("other.txt")).require("not ignored"),
        "exit 1 must map to not-ignored, not an error"
    );
}

#[test]
fn is_ignored_git_failure_is_error() {
    // Outside any repository, `check-ignore` exits 128 ("not a git
    // repository") — that must be an error, never "not ignored".
    let outside = Fixture::new().build().require("fixture without git");
    let err = git::is_ignored(outside.root(), &outside.path("whatever.txt"))
        .expect_err("check-ignore outside a repo must error");
    match err {
        GitError::Git { code, .. } => assert!(code >= 128, "expected a ≥128 exit, got {code}"),
        other => panic!("expected GitError::Git, got: {other}"),
    }
}

#[test]
fn is_ignored_spawn_failure_is_error() {
    // Parent: run the child with an empty PATH so git cannot spawn —
    // spawn failure must be an error, not "not ignored" (env mutated only
    // in the child process; parallel test threads).
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    let exe = std::env::current_exe().require("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", "is_ignored_spawn_failure_child", "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env("GENESIS_GIT_TEST_REPO", fixture.root())
        .env("PATH", "")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .require("spawn child test");
    assert!(
        output.status.success(),
        "child test with empty PATH must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn is_ignored_spawn_failure_child() {
    // Child of is_ignored_spawn_failure_is_error. Harmless standalone.
    if std::env::var_os(CHILD_SENTINEL).is_none() {
        return;
    }
    let repo =
        std::path::PathBuf::from(std::env::var("GENESIS_GIT_TEST_REPO").require("child repo path"));
    let err =
        git::is_ignored(&repo, &repo.join("whatever.txt")).expect_err("spawn failure must error");
    assert!(
        matches!(err, GitError::Spawn { .. }),
        "expected GitError::Spawn, got: {err}"
    );
}
