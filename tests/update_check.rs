//! Integration tests for the feature-gated `update_check` module.
//!
//! Strategy: all HTTP interactions go against a local throwaway TCP server,
//! so tests are hermetic (no crates.io access). Env-var skip behavior lives
//! in a separate test binary (tests/update_check_env.rs) because env vars are
//! process-global and integration tests within one binary run in parallel.

#![cfg(feature = "update-check")]

mod common;

use common::TestUnwrap;

#[path = "update_check/util.rs"]
mod update_check_util;

#[allow(unused_imports)]
use update_check_util::*;

use genesis::update_check::check_with;

#[test]
fn fresh_cache_returns_cached_update_without_http() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();
    write_cache(&dir, "mytool", now_secs(), Some("2.0.0"), TTL_SECS);

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("cached update should surface");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(info.current, "1.0.0");
    assert_eq!(
        server.request_count(),
        0,
        "fresh cache must make zero HTTP calls"
    );
}

/// genesis-u47: a cached `latest` older than the installed version must NOT
/// surface as an update — that reads as "downgrade available".
/// Cached latest older than (or equal to) installed never suggests a
/// downgrade and never touches the network — semver, calendar, and
/// equal-version schemes (genesis-u47).
#[test]
fn cached_latest_older_than_installed_returns_none() {
    let schemes = [
        ("1.2.3", "2.0.0", "semver scheme"),
        ("2026.9.28", "2026.10.4", "calendar scheme"),
        ("1.0.0", "1.0.0", "equal versions"),
    ];
    for (cached, installed, scheme) in schemes {
        let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[(cached, false)]));
        let dir = temp_cache_dir();
        write_cache(&dir, "mytool", now_secs(), Some(cached), TTL_SECS);

        let result = check_with("mytool", installed, &dir, &server.url);

        assert!(
            result.is_none(),
            "{scheme}: cached {cached} vs installed {installed} must not suggest an update"
        );
        assert_eq!(
            server.request_count(),
            0,
            "{scheme}: fresh cache must be trusted without HTTP"
        );
    }
}

#[test]
fn stale_cache_fetches_and_rewrites_cache() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();
    write_cache(
        &dir,
        "mytool",
        now_secs() - TTL_SECS - 1,
        Some("1.5.0"),
        TTL_SECS,
    );

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("newer stable version should surface");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(
        server.request_count(),
        1,
        "stale cache must trigger exactly one fetch"
    );

    let cache: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).require("value"))
            .require("value");
    assert_eq!(cache["latest"], "2.0.0");
    assert!(cache["checked_at"].as_u64().require("as_u64") > now_secs() - 60);
}

#[test]
fn missing_cache_fetches() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("update should surface on first run");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn http_error_is_fail_silent() {
    let server = TestServer::spawn("HTTP/1.1 500 Internal Server Error", "{}");
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    assert!(
        result.is_none(),
        "HTTP errors must degrade to None, not panic"
    );
}

#[test]
fn connection_refused_is_fail_silent() {
    // Port 1 on loopback: nothing listens there.
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, "http://127.0.0.1:1");

    assert!(
        result.is_none(),
        "unreachable server must degrade to None, not panic"
    );
}

#[test]
fn corrupt_cache_is_treated_as_missing() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();
    std::fs::write(dir.join("mytool.json"), "{{{ not json").require("value");

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("corrupt cache must fall back to a live fetch");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn unwritable_cache_dir_still_returns_result_without_panicking() {
    // A file where a directory should be: cache read AND write both fail.
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let blocker = temp_cache_dir().join("blocker");
    std::fs::write(&blocker, "not a dir").require("write");

    let result = check_with("mytool", "1.0.0", &blocker, &server.url);

    let info = result.require("cache write failure must not swallow the result");
    assert_eq!(info.latest, "2.0.0");
}

#[test]
fn yanked_and_prerelease_versions_are_filtered_out() {
    // 2.0.0-alpha.1 (prerelease) and 1.5.0 (yanked) must not be recommended
    // over the installed 1.0.0 — only 1.0.0 itself remains → no update.
    let server = TestServer::spawn(
        "HTTP/1.1 200 OK",
        crates_io_payload(&[("2.0.0-alpha.1", false), ("1.5.0", true), ("1.0.0", false)]),
    );
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    assert!(
        result.is_none(),
        "yanked and prerelease versions must never be suggested as updates"
    );
}
