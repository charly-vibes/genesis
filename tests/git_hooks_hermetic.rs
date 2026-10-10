//! Contract tests for `genesis::git_hooks` hermeticity under hook-context
//! environment injection.
//!
//! Same hazard class as the Fixture guard (genesis-z0r blocker): `git
//! commit` exports `GIT_DIR` into hook environments, and the pre-commit
//! testaruda hook runs the whole suite — so `git_hooks` queries that
//! spawn `git -C <root> config` would read the *real* repository's
//! config instead of the fixture's. Queries anchored at an explicit
//! `root` must discover that repository from `root` alone.

mod common;
use std::path::PathBuf;
use std::process::Command;

use genesis::fixture::Fixture;
use genesis::git_hooks::{self, HookName};

/// Marks a test process as a poisoned-env child of a parent test.
const CHILD_SENTINEL: &str = "GENESIS_GIT_HOOKS_ENV_CHILD";

/// Poison `GIT_DIR` location for the child (inside the system temp dir so
/// nothing outside it is touched even if a hijack succeeds).
fn poison_git_dir() -> PathBuf {
    std::env::temp_dir().join("genesis-git-hooks-poison-git-dir")
}

#[test]
fn git_hooks_queries_ignore_hook_injected_git_env() {
    let exe = std::env::current_exe().expect("locate test binary");
    let output = Command::new(exe)
        .args(["--exact", "git_hooks_hermetic_child", "--nocapture"])
        .env(CHILD_SENTINEL, "1")
        .env("GIT_DIR", poison_git_dir())
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test `git_hooks_hermetic_child` must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn git_hooks_hermetic_child() {
    // Child of git_hooks_queries_ignore_hook_injected_git_env. Harmless
    // standalone: early-returns without the child sentinel.
    if !std::env::var_os(CHILD_SENTINEL).is_some_and(|v| !v.is_empty()) {
        return;
    }

    let fixture = Fixture::new().with_git_init().build().expect("fixture");
    // Set a local hooksPath in the fixture repo (fixture-run git is
    // hermetic, so this targets the fixture).
    let out = fixture
        .run(&["git", "config", "core.hooksPath", ".husky"])
        .expect("run git config in fixture");
    assert!(out.success(), "git config failed: {}", out.stderr);

    // The query is anchored at the fixture root: it must read the
    // fixture's config, never the poisoned GIT_DIR.
    let hooks_dir = git_hooks::resolve_hooks_dir(fixture.root())
        .expect("resolve_hooks_dir must not fail under a poisoned GIT_DIR");
    assert_eq!(
        hooks_dir,
        fixture.root().join(".husky"),
        "resolve_hooks_dir must honor the fixture repo's core.hooksPath"
    );

    // install/uninstall go through the same config query.
    git_hooks::install(
        fixture.root(),
        HookName::PreCommit,
        "# genesis-test",
        "#!/bin/sh\n",
    )
    .expect("install must not fail under a poisoned GIT_DIR");
    assert!(
        fixture.root().join(".husky/pre-commit").exists(),
        "hook must be installed into the fixture's hooks dir"
    );
}
