//! Contract tests for `Fixture` git-operation hermeticity.
//!
//! Spec-adjacent guard (genesis-z0r blocker): `git commit` exports
//! `GIT_DIR` (and friends) into hook environments, and the pre-commit
//! testaruda hook runs the whole test suite — so any `Fixture` git
//! operation that inherits the process environment can be hijacked onto
//! the *real* repository (observed: a hook-time cargo test ran
//! `git commit -m initial` against the worktree's branch). Fixture git
//! operations must strip hook-context variables and operate on the
//! fixture's own repository, always.
//!
//! Env probes run in spawned child processes (parallel test threads must
//! never mutate the process environment).

use std::path::PathBuf;
use std::process::Command;

use genesis::fixture::Fixture;

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_FIXTURE_ENV_CHILD";

/// Poison `GIT_DIR` location for the child (inside the system temp dir so
/// nothing outside it is touched even if a hijack succeeds).
fn poison_git_dir() -> PathBuf {
    std::env::temp_dir().join("genesis-fixture-poison-git-dir")
}

/// Repo path when running as a child; `None` (and a harmless no-op) when
/// the test runs standalone in the normal suite.
fn child_marker() -> Option<()> {
    std::env::var_os(CHILD_SENTINEL).map(|_| ())
}

#[test]
fn fixture_git_ops_ignore_hook_injected_git_env() {
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", "fixture_hermetic_env_child", "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env("GIT_DIR", poison_git_dir())
        .env("GIT_INDEX_FILE", poison_git_dir().join("index"))
        .env("GIT_WORK_TREE", std::env::temp_dir())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test `fixture_hermetic_env_child` must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fixture_hermetic_env_child() {
    // Child of fixture_git_ops_ignore_hook_injected_git_env: with GIT_DIR,
    // GIT_INDEX_FILE, and GIT_WORK_TREE poisoned, every Fixture git
    // operation must still target the fixture's own repository. Harmless
    // standalone: early-returns without the child sentinel.
    if child_marker().is_none() {
        return;
    }

    // `with_git_init` runs `git init` + `git add -A` + `git commit` — the
    // exact hijack sequence observed at hook time. It must build cleanly
    // against the fixture repo, never the poisoned GIT_DIR.
    let fixture = Fixture::new().with_git_init().build().expect("fixture");

    // And arbitrary fixture-run git commands must also stay in the
    // fixture repo: `git commit` under the poisoned env would otherwise
    // land on whatever GIT_DIR points at.
    let out = fixture
        .run(&["git", "rev-parse", "--show-toplevel"])
        .expect("run git in fixture");
    assert!(out.success(), "rev-parse failed: {}", out.stderr);
    assert_eq!(
        std::path::Path::new(out.stdout.trim()),
        fixture.root(),
        "fixture-run git must resolve to the fixture repo, not the poisoned GIT_DIR"
    );
}
