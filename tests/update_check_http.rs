//! HTTP-transport, header, and notice tests for the feature-gated
//! `update_check` module (split from update_check.rs for file-size hygiene).

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

use genesis::update_check::{UpdateInfo, check_with, notice};
use std::time::Duration;

#[test]
fn calendar_versions_compare_without_semver_assumptions() {
    // Calendar versioning: 2026.10.12 is newer than 2026.9.28.
    let server = TestServer::spawn(
        "HTTP/1.1 200 OK",
        crates_io_payload(&[("2026.10.12", false), ("2026.9.28", false)]),
    );
    let dir = temp_cache_dir();

    let result = check_with("mytool", "2026.9.28", &dir, &server.url);

    let info = result.require("calendar-version update should surface");
    assert_eq!(info.latest, "2026.10.12");
}

#[test]
fn rate_limit_response_extends_ttl() {
    // 429 → fail silent, and the doubled TTL must suppress the next check.
    let server = TestServer::spawn("HTTP/1.1 429 Too Many Requests", "{}");
    let dir = temp_cache_dir();

    let first = check_with("mytool", "1.0.0", &dir, &server.url);
    assert!(first.is_none());

    let second = check_with("mytool", "1.0.0", &dir, &server.url);
    assert!(second.is_none());
    assert_eq!(
        server.request_count(),
        1,
        "rate-limited response must extend the cache TTL so the retry is suppressed"
    );

    let cache: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).require("value"))
            .require("value");
    assert_eq!(cache["ttl_secs"].as_u64().require("as_u64"), TTL_SECS * 2);
}

#[test]
fn equal_timestamps_select_the_first_element() {
    // crates.io returns versions newest-first; selection must take the first
    // stable entry, never compare created_at strings (ties would resolve to
    // the LAST element = the OLDEST version). CORR-001.
    let server = TestServer::spawn(
        "HTTP/1.1 200 OK",
        crates_io_payload_same_timestamp(&[("2.0.0", false), ("1.9.0", false)]),
    );
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("first stable version must be selected");
    assert_eq!(info.latest, "2.0.0", "must select the FIRST (newest) entry");
    assert_eq!(
        info.published_at.as_deref(),
        Some("2026-09-01T00:00:00.000000+00:00")
    );
}

#[test]
fn invalid_crate_name_returns_none_without_http() {
    // crate_name is interpolated into both the URL and the cache filename;
    // traversal sequences must be rejected before any IO. CORR-002.
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let result = check_with("mytool/../evil", "1.0.0", &dir, &server.url);

    assert!(
        result.is_none(),
        "path-traversal crate names must be rejected"
    );
    assert_eq!(
        server.request_count(),
        0,
        "invalid names must never hit the API"
    );
    assert!(
        !dir.join("evil.json").exists() && !dir.join("mytool").exists(),
        "invalid names must not write any cache file"
    );
}

#[test]
fn empty_crate_name_returns_none_without_http() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let result = check_with("", "1.0.0", &dir, &server.url);

    assert!(result.is_none());
    assert_eq!(server.request_count(), 0);
}

#[test]
fn transport_backoff_preserves_known_good_latest() {
    // A failed fetch must not overwrite a previously known latest with null:
    // the backoff entry keeps the stale-but-real version. CORR-003.
    let server = TestServer::spawn("HTTP/1.1 500 Internal Server Error", "{}");
    let dir = temp_cache_dir();
    write_cache(
        &dir,
        "mytool",
        now_secs() - TTL_SECS - 1,
        Some("1.5.0"),
        TTL_SECS,
    );

    let first = check_with("mytool", "1.0.0", &dir, &server.url);
    assert!(first.is_none(), "transport failure must stay fail-silent");

    let cache: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).require("value"))
            .require("value");
    assert_eq!(
        cache["latest"], "1.5.0",
        "backoff must preserve the known-good latest"
    );
    assert_eq!(
        cache["ttl_secs"].as_u64().require("as_u64"),
        24 * 60 * 60,
        "transport backoff uses the short (1 day) window"
    );

    // The preserved latest stays usable inside the backoff window: zero HTTP.
    let second = check_with("mytool", "1.0.0", &dir, &server.url);
    let info = second.require("preserved latest must surface within backoff");
    assert_eq!(info.latest, "1.5.0");
    assert_eq!(
        server.request_count(),
        1,
        "backoff short-circuits the retry"
    );
}

#[test]
fn future_checked_at_is_treated_as_stale() {
    // Clock skew: a cache stamped in the future must not look maximally
    // fresh — now < checked_at counts as stale and forces a refetch. EDGE-002.
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();
    write_cache(&dir, "mytool", now_secs() + 3600, Some("1.0.0"), TTL_SECS);

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("future-dated cache must be refetched, not trusted");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn slow_response_within_the_total_budget_still_succeeds() {
    // A 3s response used to exceed the old 2s TOTAL_TIMEOUT. The total
    // budget must be 5s so slow links still get an answer. EDGE-003.
    let server = TestServer::spawn_fn(move |_req| {
        std::thread::sleep(Duration::from_secs(3));
        (
            "HTTP/1.1 200 OK".to_string(),
            crates_io_payload(&[("2.0.0", false)]),
        )
    });
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.require("a 3s response must fit the total timeout budget");
    assert_eq!(info.latest, "2.0.0");
}

#[test]
fn request_path_is_the_crate_endpoint() {
    // The fetch must hit GET /<crate_name> — wiring the wrong crate name
    // (e.g. wai vs wai-cli) otherwise fails invisibly. EXCL-001.
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let _ = check_with("mytool", "1.0.0", &dir, &server.url);

    let raw = server.last_request();
    assert!(
        raw.starts_with("GET /mytool HTTP/1.1"),
        "request must be GET /<crate_name>, got: {raw}"
    );
}

#[test]
fn debug_signal_child() {
    // Child of debug_signal_emits_one_stderr_line: a failed fetch under
    // GENESIS_UPDATE_CHECK_DEBUG prints one stderr line with the reason.
    // Harmless standalone: 404 stays fail-silent. EXCL-002.
    let server = TestServer::spawn(
        "HTTP/1.1 404 Not Found",
        "{\"errors\":[{\"detail\":\"crate `mytool` does not exist\"}]}",
    );
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    assert!(result.is_none());
    assert_eq!(server.request_count(), 1);
}

#[test]
fn debug_signal_emits_one_stderr_line() {
    // Runs debug_signal_child as a subprocess with the debug env var set and
    // --nocapture so the eprintln reaches the piped stderr, then asserts the
    // line names update-check and the failure reason (HTTP 404). EXCL-002.
    let exe = std::env::current_exe().require("locate test binary");
    let output = std::process::Command::new(exe)
        .args(["--exact", "debug_signal_child", "--nocapture"])
        .env("GENESIS_UPDATE_CHECK_DEBUG", "1")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .require("spawn child test");
    assert!(output.status.success(), "child test must pass");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("update-check"),
        "debug line must identify the update-check mechanism, got: {stderr}"
    );
    assert!(
        stderr.contains("404"),
        "debug line must carry the failure reason, got: {stderr}"
    );
}

#[test]
fn request_carries_genesis_user_agent() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let _ = check_with("mytool", "1.0.0", &dir, &server.url);

    assert!(
        server.last_request_contains("genesis"),
        "crates.io requires a descriptive User-Agent; request must identify genesis"
    );
}

#[test]
fn notice_is_a_single_actionable_line() {
    let info = UpdateInfo {
        crate_name: "mytool".to_string(),
        latest: "2026.10.12".to_string(),
        current: "2026.9.28".to_string(),
        published_at: None,
    };

    let n = notice(&info);

    assert_eq!(n.lines().count(), 1, "notice must be exactly one line");
    assert!(n.contains("mytool"), "notice must name the tool");
    assert!(n.contains("2026.10.12"), "notice must show latest version");
    assert!(
        n.contains("2026.9.28"),
        "notice must show installed version"
    );
    assert!(
        n.contains("cargo install mytool"),
        "notice must include the fix command"
    );
}
