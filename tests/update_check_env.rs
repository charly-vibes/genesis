//! Env-var skip behavior for `update_check::check`.
//!
//! Separate test binary on purpose: integration tests within one binary run in
//! parallel threads, and env vars are process-global — setting CI=true here
//! would race with HTTP tests in tests/update_check.rs. `check()` skips BEFORE
//! any IO, so these tests need no server and assert only the None result plus
//! an untouched cache dir.

#![cfg(feature = "update-check")]

use genesis::update_check::{cache_path, check};
use std::sync::{Mutex, MutexGuard, OnceLock};

/// Serializes env mutation across this binary's tests: only one test holds the
/// lock at a time, so no other thread in this process reads or writes env
/// concurrently with a mutation.
fn env_lock() -> MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

#[test]
fn ci_env_skips_check_entirely() {
    let _guard = env_lock();
    // SAFETY: env_lock() guarantees no other test thread mutates or reads env
    // concurrently; the process otherwise leaves these vars untouched.
    unsafe { std::env::set_var("CI", "true") };
    let result = check("genesis-update-check-env-test", "0.0.1");
    unsafe { std::env::remove_var("CI") };

    assert!(
        result.is_none(),
        "CI=true must skip the check before any IO"
    );
    assert!(
        cache_path("genesis-update-check-env-test").is_none_or(|p| !p.exists()),
        "skipped check must not write cache"
    );
}

#[test]
fn opt_out_env_var_skips_check_entirely() {
    let _guard = env_lock();
    // SAFETY: see env_lock() comment above.
    unsafe { std::env::set_var("GENESIS_NO_UPDATE_CHECK", "1") };
    let result = check("genesis-update-check-env-test", "0.0.1");
    unsafe { std::env::remove_var("GENESIS_NO_UPDATE_CHECK") };

    assert!(
        result.is_none(),
        "GENESIS_NO_UPDATE_CHECK must skip the check"
    );
}

#[test]
fn empty_opt_out_value_does_not_skip() {
    let _guard = env_lock();
    // SAFETY: see env_lock() comment above.
    unsafe { std::env::set_var("GENESIS_NO_UPDATE_CHECK", "") };
    // Hermetic: pin XDG_CACHE_HOME to a fresh temp dir so the fetch path's
    // backoff write (and the crates.io hit that precedes it) can never touch
    // the user's real ~/.cache (genesis-4mq EDGE-001).
    let tmp = tempfile::TempDir::new().expect("temp cache home");
    unsafe { std::env::set_var("XDG_CACHE_HOME", tmp.path()) };
    // An empty value means "not set", so the check proceeds to the fetch
    // path; a nonexistent crate 404s and fails silently.
    let result = check("genesis-update-check-env-test-nonexistent-crate", "0.0.1");
    let p = cache_path("genesis-update-check-env-test-nonexistent-crate")
        .expect("XDG_CACHE_HOME set so cache_path must resolve");
    unsafe { std::env::remove_var("XDG_CACHE_HOME") };
    unsafe { std::env::remove_var("GENESIS_NO_UPDATE_CHECK") };

    assert!(result.is_none());
    assert!(
        p.starts_with(tmp.path()),
        "cache writes must stay inside the hermetic XDG_CACHE_HOME"
    );
}

// genesis-39r: XDG_CACHE_HOME IS the cache root (XDG Base Directory spec),
// matching the doc comment and feedback/scratch.rs precedent — the base must
// NOT get a .cache component appended when XDG_CACHE_HOME is set.
#[test]
fn cache_path_honors_xdg_cache_home_as_cache_root() {
    let _guard = env_lock();
    let tmp = tempfile::TempDir::new().expect("temp cache home");
    unsafe { std::env::set_var("XDG_CACHE_HOME", tmp.path()) };
    let p = cache_path("somecrate");
    unsafe { std::env::remove_var("XDG_CACHE_HOME") };

    assert_eq!(
        p.as_deref(),
        Some(
            tmp.path()
                .join("genesis")
                .join("update-check")
                .join("somecrate.json")
        )
        .as_deref(),
        "XDG_CACHE_HOME must be used as the cache root directly, no .cache suffix"
    );
}

#[test]
fn cache_path_falls_back_to_home_cache_when_xdg_unset() {
    let _guard = env_lock();
    let tmp = tempfile::TempDir::new().expect("temp home");
    unsafe { std::env::remove_var("XDG_CACHE_HOME") };
    unsafe { std::env::set_var("HOME", tmp.path()) };
    let p = cache_path("somecrate");
    unsafe { std::env::remove_var("HOME") };

    assert_eq!(
        p.as_deref(),
        Some(
            tmp.path()
                .join(".cache")
                .join("genesis")
                .join("update-check")
                .join("somecrate.json")
        )
        .as_deref(),
        "unset XDG_CACHE_HOME falls back to $HOME/.cache"
    );
}
