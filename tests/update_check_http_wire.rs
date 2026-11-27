//! Wire-level header tests for the feature-gated `update_check` module:
//! default and caller-overridden User-Agent (genesis-3i5), plus the
//! GENESIS_UPDATE_CHECK_DEBUG stderr signal (EXCL-002).

#![cfg(feature = "update-check")]

mod common;

use common::TestUnwrap;

#[path = "update_check/util.rs"]
mod update_check_util;

#[allow(unused_imports)]
use update_check_util::*;

use genesis::update_check::check_with;

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

    let result = check_with("mytool", "1.0.0", &dir, &server.url, None);

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

    let _ = check_with("mytool", "1.0.0", &dir, &server.url, None);

    assert!(
        server.last_request_contains("User-Agent: genesis-vibes/"),
        "default UA must stay genesis-vibes; request must identify genesis"
    );
}

/// genesis-3i5: callers may override the User-Agent sent to crates.io.
#[test]
fn user_agent_override_reaches_the_wire() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let _ = check_with("mytool", "1.0.0", &dir, &server.url, Some("mytool/9.9.9"));

    let raw = server.last_request();
    assert!(
        raw.contains("User-Agent: mytool/9.9.9"),
        "caller UA override must reach the wire, got: {raw}"
    );
    assert!(
        !raw.contains("genesis-vibes"),
        "override must replace, not mix with, the default UA"
    );
}
