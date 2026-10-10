//! Contract tests for `genesis::git` repo-root detection.
//!
//! Spec: walk-based repo-root detection (worktree `.git` file, nested
//! subdirectory, success inside `.git` dirs, typed `NotInRepo` outside,
//! independence from `GIT_DIR`).
//!
//! `GIT_DIR` independence is asserted in spawned child processes: cargo
//! runs tests in parallel threads, so the test process's own environment
//! must never be mutated (poison the child's env via `.env()` instead).

mod common;
use std::path::{Path, PathBuf};
use std::process::Command;

use genesis::fixture::Fixture;
use genesis::git;

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_GIT_TEST_CHILD";

/// Path of the fixture repository the parent test hands to its child.
const CHILD_REPO: &str = "GENESIS_GIT_TEST_REPO";

/// Repo path when running as a child; `None` (and a harmless no-op) when
/// the test runs standalone in the normal suite.
fn child_repo() -> Option<PathBuf> {
    std::env::var_os(CHILD_SENTINEL)?;
    Some(PathBuf::from(
        std::env::var(CHILD_REPO).expect("child must receive the fixture repo path"),
    ))
}

#[test]
fn repo_root_resolves_worktree_git_file() {
    let main = Fixture::new()
        .with_git_init()
        .build()
        .expect("main fixture");
    let parent = Fixture::new().build().expect("worktree parent fixture");
    let wt = parent.root().join("linked");
    let out = main
        .run(&["git", "worktree", "add", wt.to_str().unwrap(), "HEAD"])
        .expect("run git worktree add");
    assert!(out.success(), "git worktree add failed: {}", out.stderr);
    assert!(
        wt.join(".git").is_file(),
        "worktree `.git` must be a link file"
    );

    assert_eq!(
        git::repo_root_from(&wt).expect("worktree root"),
        wt,
        "a `.git` FILE must count as a repository root"
    );

    let nested = wt.join("a/b");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    assert_eq!(
        git::repo_root_from(&nested).expect("nested worktree root"),
        wt,
        "nested subdirectory inside a worktree must resolve to the worktree root"
    );
}

#[test]
fn repo_root_resolves_nested_subdirectory() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let nested = fixture.root().join("a/b");
    std::fs::create_dir_all(&nested).expect("create nested dir");
    assert_eq!(
        git::repo_root_from(&nested).expect("nested root"),
        fixture.root(),
        "two levels below the root must still resolve to the repository root"
    );
}

#[test]
fn repo_root_succeeds_inside_git_dir() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let inside = fixture.root().join(".git");
    assert_eq!(
        git::repo_root_from(&inside).expect("inside .git"),
        fixture.root(),
        "starting the walk inside a `.git` directory must succeed"
    );
}

#[test]
fn repo_root_not_in_repo_is_typed_error() {
    let fixture = Fixture::new().build().expect("fixture without git");
    let err = git::repo_root_from(fixture.root()).expect_err("tempdir is not a repo");
    match err {
        git::GitError::NotInRepo { start } => {
            assert_eq!(start, fixture.root(), "error must carry the start dir");
        }
        other => panic!("expected NotInRepo, got: {other}"),
    }
    // And it must not panic when the walk reaches the filesystem root.
    let err = git::repo_root_from(Path::new("/")).expect_err("filesystem root is not a repo");
    assert!(matches!(err, git::GitError::NotInRepo { .. }));
}

#[test]
fn repo_root_walk_ignores_git_dir_env() {
    // Parent: spawn the child test with GIT_DIR poisoned to a nonexistent
    // path. The child asserts the walk still succeeds — proving root
    // detection is independent of GIT_DIR without mutating this process's
    // own env (racy across parallel test threads).
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", "repo_root_child_poisoned_git_dir", "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env(CHILD_REPO, fixture.root())
        .env("GIT_DIR", "/definitely/not/a/repo/.git")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test with poisoned GIT_DIR must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn repo_root_child_poisoned_git_dir() {
    // Child of repo_root_walk_ignores_git_dir_env. Harmless standalone:
    // early-returns without the child sentinel.
    let Some(repo) = child_repo() else {
        return;
    };
    assert_eq!(
        git::repo_root_from(&repo).expect("walk must ignore GIT_DIR"),
        repo,
        "walk must resolve the real root even with GIT_DIR poisoned"
    );
    let inner = repo.join(".git");
    assert_eq!(
        git::repo_root_from(&inner).expect("inside .git with poisoned GIT_DIR"),
        repo,
        "walk from inside .git must also ignore GIT_DIR"
    );
}
