//! Fixture git-operation hermeticity and build-error hygiene.
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
//! Also guarded here (genesis-352): seeded repos are isolated from host
//! git config (commit.gpgsign, init.defaultBranch), failed builds clean
//! their temp root, and `assert_file_contains` surfaces read errors.
//!
//! Env probes run in spawned child processes (parallel test threads must
//! never mutate the process environment).

mod common;
use std::path::{Path, PathBuf};
use std::process::Command;

use genesis::fixture::{Fixture, FixtureError};

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

/// Spawn one child test process as its own binary with env vars set, in
/// a separate process so parallel test threads never mutate the shared
/// process environment. `sentinel` marks the child; `vars` carries the
/// extra hostile environment. Panics unless the child test passes.
fn spawn_child_test(test: &str, sentinel: &str, vars: &[(&str, &Path)]) -> std::process::Output {
    let exe = std::env::current_exe().expect("locate test binary");
    let mut cmd = Command::new(exe);
    cmd.args(["--exact", test, "--nocapture"])
        .env(sentinel, "1");
    for (name, value) in vars {
        cmd.env(name, value);
    }
    let output = cmd
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
    assert!(
        output.status.success(),
        "child test `{test}` must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Write a file, panicking with `why` context when it fails.
fn write_file(path: &Path, contents: impl AsRef<[u8]>, why: &str) {
    if let Err(e) = std::fs::write(path, contents) {
        panic!("{why}: {e}");
    }
}

#[test]
fn fixture_git_ops_ignore_hook_injected_git_env() {
    let poison = poison_git_dir();
    let output = spawn_child_test(
        "fixture_hermetic_env_child",
        CHILD_SENTINEL,
        &[
            ("GIT_DIR", poison.as_path()),
            ("GIT_INDEX_FILE", poison.join("index").as_path()),
            ("GIT_WORK_TREE", std::env::temp_dir().as_path()),
        ],
    );
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

/// Marks a test process as a hostile-host-config child of a parent test.
const HOST_CONFIG_CHILD: &str = "GENESIS_FIXTURE_HOST_CONFIG_CHILD";

/// Host git config a user might have: signing commits with a (bogus) key
/// and a customized init.defaultBranch. Without config isolation, `git
/// commit` fails on the signing key and the seeded branch is wrong.
#[test]
fn fixture_commit_survives_host_gpgsign_and_default_branch() {
    let config = std::env::temp_dir().join("genesis-fixture-host-config");
    write_file(
        &config,
        "[commit]\n\tgpgsign = true\n\
         [user]\n\tsigningkey = BOGUS-NONEXISTENT-KEY\n\
         [init]\n\tdefaultBranch = weird-host-default\n",
        "write host git config",
    );
    let output = spawn_child_test(
        "fixture_host_config_child",
        HOST_CONFIG_CHILD,
        &[("GIT_CONFIG_GLOBAL", config.as_path())],
    );
    assert!(
        output.status.success(),
        "child test `fixture_host_config_child` must pass:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_file(&config);
}

/// Child of fixture_commit_survives_host_gpgsign_and_default_branch:
/// GIT_CONFIG_GLOBAL points at a config with commit.gpgsign=true (and a
/// bogus signing key, so signing deterministically fails) plus a custom
/// init.defaultBranch. Seeded fixtures must still build, commit
/// unsigned, and land on `main`. Harmless standalone.
#[test]
fn fixture_host_config_child() {
    if std::env::var_os(HOST_CONFIG_CHILD).is_none() {
        return;
    }

    let fixture = match Fixture::new().with_git_init().build() {
        Ok(fixture) => fixture,
        Err(e) => panic!("fixture build failed under hostile git config: {e}"),
    };
    let head = std::fs::read_to_string(fixture.path(".git/HEAD")).expect("read .git/HEAD");
    assert_eq!(
        head.trim(),
        "ref: refs/heads/main",
        "seeded repo must be on pinned `main`, not the host init.defaultBranch"
    );
    // The commit must exist and be unsigned (commit.gpgsign=false override).
    let out = fixture
        .run(&["git", "cat-file", "-p", "HEAD"])
        .expect("run git in fixture");
    assert!(out.success(), "cat-file failed: {}", out.stderr);
    assert!(
        !out.stdout.contains("gpgsig"),
        "seeded commit must not be signed"
    );
}

#[test]
fn failed_build_cleans_its_temp_root() {
    // Markers are created before files, so a file colliding with an
    // existing marker directory makes `build` fail after the root exists.
    let Err(err) = Fixture::new()
        .with_marker("occupied")
        .with_file("occupied", "collision")
        .build()
    else {
        panic!("build must fail on file/dir collision");
    };
    let FixtureError::Io { path, message } = &err else {
        panic!("expected an Io error, got: {err:?}");
    };
    assert!(
        message.contains("occupied"),
        "error must name the failing entry: {message}"
    );
    let root = path
        .parent()
        .expect("failing entry must have a parent (the temp root)");
    assert!(
        !root.exists(),
        "failed build must clean its temp root, but {} remains",
        root.display()
    );
}

#[test]
fn assert_file_contains_surfaces_read_errors() {
    let fixture = Fixture::new()
        .with_file("nonutf8.bin", "")
        .build()
        .expect("fixture");
    // Invalid UTF-8 makes read_to_string fail while the path still exists
    // and is a file, so the old `unwrap_or_else(|_| String::new())` would
    // mask it as "empty content".
    write_file(
        &fixture.path("nonutf8.bin"),
        [0xFF, 0xFE, 0xFF],
        "write nonutf8",
    );

    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        fixture.assert_file_contains("nonutf8.bin", "anything");
    }));
    let Err(panic) = result else {
        panic!("assert_file_contains must fail on unreadable files");
    };
    let message = panic
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| panic.downcast_ref::<&str>().copied())
        .unwrap_or_default();
    assert!(
        message.starts_with("failed to read fixture file"),
        "panic must surface the underlying read error, got: {message}"
    );
    assert!(
        message.contains("stream did not contain valid UTF-8"),
        "panic must include the io error itself, got: {message}"
    );
}
