//! Contract tests for single-path status with porcelain XY mapping.
//!
//! Spec: single-path status (`git status --porcelain -- <path>` for
//! exactly one path): `??` → untracked, non-blank Y → dirty, non-blank
//! X → staged (after the dirty check), unmerged entries map to dirty
//! (documented), a clean path → `None`, git failure → typed error.

mod common;

use common::TestUnwrap;
use std::path::Path;

use genesis::fixture::Fixture;
use genesis::git::{self, GitError, PathStatus};

/// Run a command in the fixture and assert it exited zero.
fn run_ok(fixture: &Fixture, args: &[&str]) {
    let out = fixture.run(args).require("spawn command");
    assert!(out.success(), "{args:?} failed: {}", out.stderr);
}

/// Write `content` at `name` in the fixture and return its git blob hash
/// (for forging index entries without a real merge).
fn write_blob(fixture: &Fixture, name: &str, content: &str) -> String {
    std::fs::write(fixture.path(name), content).require("write blob");
    let out = fixture
        .run(&["git", "hash-object", "-w", name])
        .require("hash-object");
    out.stdout.trim().to_string()
}

#[test]
fn path_status_clean_committed_path_is_none() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    // `.gitkeep` is committed by the fixture seed and left untouched.
    let status = git::path_status(fixture.root(), Path::new(".gitkeep")).require("path_status");
    assert_eq!(status, None, "a clean tracked path must report no change");
}

#[test]
fn path_status_untracked_is_untracked() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("new.txt"), "content").require("write file");
    let status = git::path_status(fixture.root(), Path::new("new.txt")).require("path_status");
    assert_eq!(status, Some(PathStatus::Untracked));
}

#[test]
fn path_status_worktree_modification_is_dirty() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitkeep"), "modified").require("modify tracked file");
    let status = git::path_status(fixture.root(), Path::new(".gitkeep")).require("path_status");
    assert_eq!(status, Some(PathStatus::Dirty));
}

#[test]
fn path_status_staged_modification_is_staged() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitkeep"), "staged content").require("modify tracked file");
    run_ok(&fixture, &["git", "add", ".gitkeep"]);
    let status = git::path_status(fixture.root(), Path::new(".gitkeep")).require("path_status");
    assert_eq!(status, Some(PathStatus::Staged));
}

#[test]
fn path_status_unmerged_maps_to_dirty() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    // Forge unmerged index entries (stages 2 and 3) without a real merge:
    // `git status --porcelain` reports `UU` for the path.
    let ours = write_blob(&fixture, "conflicted.txt", "ours");
    let theirs = write_blob(&fixture, "conflicted.txt", "theirs");
    run_ok(
        &fixture,
        &[
            "sh",
            "-c",
            &format!(
                "printf '100644 {ours} 2\\tconflicted.txt\\n100644 {theirs} 3\\tconflicted.txt\\n' \
                 | git update-index --index-info"
            ),
        ],
    );

    let status =
        git::path_status(fixture.root(), Path::new("conflicted.txt")).require("path_status");
    assert_eq!(
        status,
        Some(PathStatus::Dirty),
        "unmerged states must map to dirty (documented)"
    );
}

#[test]
fn path_status_worktree_change_wins_over_staged() {
    // `MM`: staged AND worktree-modified. The documented precedence
    // reports the worktree state (dirty) when both columns are set.
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path(".gitkeep"), "staged content").require("stage content");
    run_ok(&fixture, &["git", "add", ".gitkeep"]);
    std::fs::write(fixture.path(".gitkeep"), "worktree content").require("modify again");
    let status = git::path_status(fixture.root(), Path::new(".gitkeep")).require("path_status");
    assert_eq!(status, Some(PathStatus::Dirty));
}

#[test]
fn path_status_accepts_absolute_path_under_root() {
    let fixture = Fixture::new().with_git_init().build().require("fixture");
    std::fs::write(fixture.path("new.txt"), "content").require("write file");
    let status = git::path_status(fixture.root(), &fixture.path("new.txt")).require("path_status");
    assert_eq!(status, Some(PathStatus::Untracked));
}

#[test]
fn path_status_git_failure_is_typed_error() {
    // A directory with no repository at all: the spawned git fails with a
    // non-zero exit, which must surface as a typed error.
    let fixture = Fixture::new().build().require("fixture without git");
    let err = git::path_status(fixture.root(), Path::new("whatever.txt"))
        .expect_err("outside a repository must be an error");
    assert!(
        matches!(err, GitError::Git { .. }),
        "expected GitError::Git, got: {err}"
    );
}
