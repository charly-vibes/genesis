//! Contract tests for the canonical `genesis::git` runner and its
//! opt-in environment policy.
//!
//! Spec: canonical runner with opt-in env hygiene (default policy inherits
//! hook-injected variables; `StripHookContext` removes `GIT_DIR`,
//! `GIT_INDEX_FILE`, `GIT_WORK_TREE`), plus declared failure semantics
//! (non-zero exit → typed `GitError::Git`, spawn failure → `GitError::Spawn`).
//!
//! Env-variable probes run in spawned child processes: cargo runs tests in
//! parallel threads, so the test process's own environment must never be
//! mutated (poison the child's env via `.env()` instead).

use std::path::PathBuf;
use std::process::Command;

use genesis::fixture::Fixture;
use genesis::git::{self, EnvPolicy, GitError};

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_GIT_TEST_CHILD";

/// Path of the fixture repository the parent test hands to its child.
const CHILD_REPO: &str = "GENESIS_GIT_TEST_REPO";

/// Repo path when running as a child; `None` (and a harmless no-op) when
/// the test runs standalone in the normal suite.
fn child_context() -> Option<PathBuf> {
    std::env::var_os(CHILD_SENTINEL)?;
    Some(PathBuf::from(
        std::env::var(CHILD_REPO).expect("child must receive the fixture repo path"),
    ))
}

/// Build the poisoned environment a child test runs under.
///
/// `GIT_DIR` points at the real repository `.git`, `GIT_WORK_TREE` at a
/// bare directory with no repository, and `GIT_INDEX_FILE` at a corrupt
/// index file — so each variable's inheritance has an observable effect.
fn spawn_child(exact_test: &str, repo: &std::path::Path, corrupt_index: &std::path::Path) {
    let fake_worktree = std::env::temp_dir().join("genesis-git-runner-fake-worktree");
    std::fs::create_dir_all(&fake_worktree).expect("create fake worktree dir");
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", exact_test, "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env(CHILD_REPO, repo)
        .env("GENESIS_GIT_TEST_CORRUPT_INDEX", corrupt_index)
        .env("GIT_DIR", repo.join(".git"))
        .env("GIT_WORK_TREE", &fake_worktree)
        .env("GIT_INDEX_FILE", corrupt_index)
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
fn runner_default_policy_inherits_git_dir() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let corrupt = fixture.root().join("corrupt-index");
    std::fs::write(&corrupt, b"not a real index").expect("write corrupt index");
    spawn_child("runner_inherit_child", fixture.root(), &corrupt);
}

#[test]
fn runner_inherit_child() {
    // Child of runner_default_policy_inherits_git_dir: with the default
    // policy, each poisoned hook-context variable must reach git.
    // Harmless standalone: early-returns without the child sentinel.
    let Some(repo) = child_context() else {
        return;
    };
    let git_dir = std::env::var("GIT_DIR").expect("GIT_DIR poisoned in child");

    // GIT_DIR: rev-parse --git-dir must echo the poisoned value.
    let out = git::run(&repo, EnvPolicy::Inherit, &["rev-parse", "--git-dir"])
        .expect("inherit policy must pass GIT_DIR through");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        git_dir,
        "spawned git must receive GIT_DIR under the default policy"
    );

    // GIT_WORK_TREE: toplevel must be the poisoned fake worktree.
    let fake = std::env::var("GIT_WORK_TREE").expect("GIT_WORK_TREE poisoned in child");
    let out = git::run(&repo, EnvPolicy::Inherit, &["rev-parse", "--show-toplevel"])
        .expect("rev-parse --show-toplevel with GIT_DIR+GIT_WORK_TREE");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        fake,
        "spawned git must receive GIT_WORK_TREE under the default policy"
    );

    // GIT_INDEX_FILE: a corrupt index must make `diff --cached` fail —
    // proving the variable reached git.
    let err = git::run(
        &repo,
        EnvPolicy::Inherit,
        &["diff", "--cached", "--name-only"],
    )
    .expect_err("corrupt GIT_INDEX_FILE must break diff --cached under inherit");
    assert!(
        matches!(err, GitError::Git { .. }),
        "expected a typed Git error, got: {err}"
    );
}

#[test]
fn runner_strip_hook_context_removes_git_env() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let corrupt = fixture.root().join("corrupt-index");
    std::fs::write(&corrupt, b"not a real index").expect("write corrupt index");
    spawn_child("runner_strip_child", fixture.root(), &corrupt);
}

#[test]
fn runner_strip_child() {
    // Child of runner_strip_hook_context_removes_git_env: with
    // StripHookContext, none of the poisoned hook-context variables may
    // reach git. Harmless standalone.
    let Some(repo) = child_context() else {
        return;
    };

    // GIT_DIR stripped: git discovers the repo from cwd; --git-dir prints
    // the discovered path, never the poisoned absolute one.
    let out = git::run(
        &repo,
        EnvPolicy::StripHookContext,
        &["rev-parse", "--git-dir"],
    )
    .expect("strip policy must still run git in the repo");
    let printed = String::from_utf8_lossy(&out.stdout).trim().to_string();
    assert_ne!(
        printed,
        std::env::var("GIT_DIR").expect("GIT_DIR poisoned in child"),
        "GIT_DIR must not reach git under StripHookContext"
    );

    // GIT_WORK_TREE stripped: toplevel is the real repo root, not the fake.
    let out = git::run(
        &repo,
        EnvPolicy::StripHookContext,
        &["rev-parse", "--show-toplevel"],
    )
    .expect("toplevel after stripping");
    assert_eq!(
        String::from_utf8_lossy(&out.stdout).trim(),
        repo.to_string_lossy(),
        "GIT_WORK_TREE must not reach git under StripHookContext"
    );

    // GIT_INDEX_FILE stripped: the corrupt poisoned index is ignored and
    // `diff --cached` succeeds against the real index.
    git::run(
        &repo,
        EnvPolicy::StripHookContext,
        &["diff", "--cached", "--name-only"],
    )
    .expect("corrupt GIT_INDEX_FILE must be ignored under StripHookContext");
}

#[test]
fn runner_failure_yields_typed_git_error() {
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let err = git::run(
        fixture.root(),
        EnvPolicy::Inherit,
        &["rev-parse", "--verify", "genesis-no-such-ref"],
    )
    .expect_err("unknown ref must fail");
    match err {
        GitError::Git { args, code, stderr } => {
            assert!(
                args.windows(2).any(|w| w == ["rev-parse", "--verify"]),
                "error must carry the failing args, got: {args:?}"
            );
            assert_eq!(code, 128, "unknown ref exits 128");
            assert!(!stderr.is_empty(), "error must carry stderr");
        }
        other => panic!("expected GitError::Git, got: {other}"),
    }
}

#[test]
fn runner_spawn_failure_yields_spawn_error() {
    // Parent: run the child with an empty PATH so `git` cannot be found.
    // Spawning with an empty PATH only affects the child process, never
    // this one (parallel test threads).
    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", "runner_spawn_failure_child", "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env(CHILD_REPO, fixture.root())
        .env("PATH", "")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test with empty PATH must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn runner_spawn_failure_child() {
    // Child of runner_spawn_failure_yields_spawn_error. Harmless standalone.
    let Some(repo) = child_context() else {
        return;
    };
    let err = git::run(&repo, EnvPolicy::Inherit, &["status", "--porcelain"])
        .expect_err("git missing from PATH must be a spawn failure");
    assert!(
        matches!(err, GitError::Spawn { .. }),
        "expected GitError::Spawn, got: {err}"
    );
}
