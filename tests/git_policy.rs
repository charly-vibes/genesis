//! Contract tests for policy-aware named helpers in `genesis::git`.
//!
//! Spec: every named helper that spawns git exposes a `*_with_policy`
//! variant taking an explicit [`EnvPolicy`]; the plain variants keep the
//! `Inherit` default. Propagation is observable in a hook-like
//! environment: with a bogus `GIT_DIR` injected (as hook harnesses do),
//! inherit-policy calls fail while strip-policy calls succeed.
//!
//! Env-variable probes run in spawned child processes: cargo runs tests
//! in parallel threads, so the test process's own environment must never
//! be mutated (poison the child's env instead).

use std::path::{Path, PathBuf};
use std::process::Command;

use genesis::fixture::Fixture;
use genesis::git;

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_GIT_POLICY_CHILD";

/// Path of the fixture repository the parent test hands to its child.
const CHILD_REPO: &str = "GENESIS_GIT_POLICY_REPO";

/// Repo path when running as a child; `None` (and a harmless no-op) when
/// the test runs standalone in the normal suite.
fn child_context() -> Option<PathBuf> {
    std::env::var_os(CHILD_SENTINEL)?;
    Some(PathBuf::from(
        std::env::var(CHILD_REPO).expect("child must receive the fixture repo path"),
    ))
}

/// Spawn a child test with a hook-like environment: `GIT_DIR` poisoned to
/// a path that cannot be a git directory (hook harnesses inject `GIT_DIR`;
/// the bogus value makes inheritance observable as a hard failure).
fn spawn_child(exact_test: &str, repo: &std::path::Path) {
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env(CHILD_REPO, repo)
        .env("GIT_DIR", repo.join(".git/genesis-bogus-git-dir"))
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test `{exact_test}` must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn named_helpers_propagate_the_environment_policy() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    spawn_child("policy_propagation_child", fixture.root());
}

#[test]
fn policy_propagation_child() {
    // Child of named_helpers_propagate_the_environment_policy. Harmless
    // standalone: early-returns without the child sentinel.
    let Some(repo) = child_context() else {
        return;
    };
    let keep = Path::new(".gitkeep");

    // Plain variants default to Inherit: the bogus GIT_DIR reaches the
    // spawned git process, so every helper fails with a typed error.
    assert!(
        git::uncommitted_files(&repo).is_err(),
        "plain uncommitted_files must inherit GIT_DIR (default policy)"
    );
    assert!(
        git::changed_files(&repo).is_err(),
        "plain changed_files must inherit GIT_DIR (default policy)"
    );
    assert!(
        git::tracked(&repo, keep).is_err(),
        "plain tracked must inherit GIT_DIR (default policy)"
    );
    assert!(
        git::content_hash(&repo, keep).is_err(),
        "plain content_hash must inherit GIT_DIR (default policy)"
    );
    assert!(
        git::committed_content_hash(&repo, keep).is_err(),
        "plain committed_content_hash must inherit GIT_DIR (default policy)"
    );
    assert!(
        git::path_status(&repo, keep).is_err(),
        "plain path_status must inherit GIT_DIR (default policy)"
    );

    // `*_with_policy(StripHookContext)`: the bogus GIT_DIR is removed, so
    // git discovers the repository from `root` alone and every helper
    // succeeds with correct values.
    let policy = genesis::git::EnvPolicy::StripHookContext;
    assert_eq!(
        git::uncommitted_files_with_policy(repo, policy).expect("strip policy"),
        Vec::<String>::new(),
        "clean repo must report no uncommitted files"
    );
    assert_eq!(
        git::changed_files_with_policy(repo, policy).expect("strip policy"),
        Vec::<String>::new(),
        "clean repo must report no changed files"
    );
    assert!(
        git::tracked_with_policy(repo, keep, policy).expect("strip policy"),
        ".gitkeep is committed, hence tracked"
    );
    assert!(
        !git::is_ignored_with_policy(repo, keep, policy).expect("strip policy"),
        ".gitkeep is not ignored"
    );
    assert!(
        !git::content_hash_with_policy(repo, keep, policy)
            .expect("strip policy")
            .is_empty(),
        "content_hash must produce a hash"
    );
    assert!(
        !git::committed_content_hash_with_policy(repo, keep, policy)
            .expect("strip policy")
            .is_empty(),
        "committed_content_hash must produce a hash"
    );
    assert_eq!(
        git::changed_files_between_with_policy(repo, "HEAD", "HEAD", policy).expect("strip policy"),
        Vec::<String>::new(),
        "identical revisions must report no changed files"
    );
    assert_eq!(
        git::path_status_with_policy(repo, keep, policy).expect("strip policy"),
        None,
        "clean tracked path must report no change"
    );
}
