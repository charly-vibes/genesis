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
    // Cannot hit the network hermetically — but an empty value means "not set",
    // so the check proceeds to the fetch path and fails silently (offline-safe
    // assertion: just verify it does not return a fabricated update).
    let result = check("genesis-update-check-env-test-nonexistent-crate", "0.0.1");
    unsafe { std::env::remove_var("GENESIS_NO_UPDATE_CHECK") };

    assert!(result.is_none());
}
