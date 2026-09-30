//! Integration tests for the feature-gated `update_check` module.
//!
//! Strategy: all HTTP interactions go against a local throwaway TCP server,
//! so tests are hermetic (no crates.io access). Env-var skip behavior lives
//! in a separate test binary (tests/update_check_env.rs) because env vars are
//! process-global and integration tests within one binary run in parallel.

#![cfg(feature = "update-check")]

use genesis::update_check::{UpdateInfo, check_with, notice};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Minimal local HTTP server: answers every request with the canned response,
/// counts requests, and records the raw request bytes for header assertions.
struct TestServer {
    url: String,
    requests: Arc<AtomicUsize>,
    last_request: Arc<std::sync::Mutex<String>>,
}

impl TestServer {
    fn spawn(status_line: impl Into<String>, body: impl Into<String>) -> Self {
        let status_line = status_line.into();
        let body = body.into();
        Self::spawn_fn(move |_req| (status_line.clone(), body.clone()))
    }

    fn spawn_fn<F>(handler: F) -> Self
    where
        F: Fn(&str) -> (String, String) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind test server");
        let url = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(AtomicUsize::new(0));
        let last_request: Arc<std::sync::Mutex<String>> =
            Arc::new(std::sync::Mutex::new(String::new()));

        let reqs = Arc::clone(&requests);
        let value = Arc::clone(&last_request);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let mut stream = match stream {
                    Ok(s) => s,
                    Err(_) => continue,
                };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                let mut buf = Vec::new();
                let mut chunk = [0u8; 1024];
                // Read until end of headers (requests are tiny).
                loop {
                    match stream.read(&mut chunk) {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.extend_from_slice(&chunk[..n]);
                            if buf.windows(4).any(|w| w == b"\r\n\r\n") {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
                let raw = String::from_utf8_lossy(&buf).to_string();
                reqs.fetch_add(1, Ordering::SeqCst);
                *value.lock().unwrap() = raw.clone();
                let (status, body) = handler(&raw);
                let response = format!(
                    "{status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        TestServer {
            url,
            requests,
            last_request,
        }
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn last_request_contains(&self, needle: &str) -> bool {
        self.last_request.lock().unwrap().contains(needle)
    }

    fn last_request(&self) -> String {
        self.last_request.lock().unwrap().clone()
    }
}

fn crates_io_payload_same_timestamp(versions: &[(&str, bool)]) -> String {
    // All versions share one created_at — crates.io tie behavior must not
    // depend on timestamp comparison at all (genesis-4mq CORR-001).
    let items: Vec<String> = versions
        .iter()
        .map(|(num, yanked)| {
            format!(r#"{{"num":"{num}","yanked":{yanked},"created_at":"2026-09-01T00:00:00.000000+00:00"}}"#)
        })
        .collect();
    format!(r#"{{"versions":[{}]}}"#, items.join(","))
}

fn crates_io_payload(versions: &[(&str, bool)]) -> String {
    let items: Vec<String> = versions
        .iter()
        .enumerate()
        .map(|(i, (num, yanked))| {
            // Distinct, descending timestamps: item 0 is the newest, matching
            // crates.io's newest-first ordering. Deterministic for max_by.
            format!(
                r#"{{"num":"{num}","yanked":{yanked},"created_at":"2026-09-01T00:00:{:02}.000000+00:00"}}"#,
                59 - i
            )
        })
        .collect();
    format!(r#"{{"versions":[{}]}}"#, items.join(","))
}

fn temp_cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "genesis-update-check-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("create cache dir");
    dir
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

const TTL_SECS: u64 = 7 * 24 * 60 * 60;

fn write_cache(
    dir: &std::path::Path,
    crate_name: &str,
    checked_at: u64,
    latest: Option<&str>,
    ttl: u64,
) {
    let cache_file = dir.join(format!("{crate_name}.json"));
    let latest_json = match latest {
        Some(v) => format!("\"{v}\""),
        None => "null".to_string(),
    };
    std::fs::write(
        cache_file,
        format!(
            r#"{{"checked_at":{checked_at},"latest":{latest_json},"published_at":null,"ttl_secs":{ttl}}}"#
        ),
    )
    .expect("write cache");
}

#[test]
fn fresh_cache_returns_cached_update_without_http() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();
    write_cache(&dir, "mytool", now_secs(), Some("2.0.0"), TTL_SECS);

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.expect("cached update should surface");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(info.current, "1.0.0");
    assert_eq!(
        server.request_count(),
        0,
        "fresh cache must make zero HTTP calls"
    );
}

#[test]
fn fresh_cache_with_no_update_returns_none_without_http() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("1.0.0", false)]));
    let dir = temp_cache_dir();
    write_cache(&dir, "mytool", now_secs(), Some("1.0.0"), TTL_SECS);

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    assert!(result.is_none());
    assert_eq!(server.request_count(), 0);
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

    let info = result.expect("newer stable version should surface");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(
        server.request_count(),
        1,
        "stale cache must trigger exactly one fetch"
    );

    let cache: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).unwrap()).unwrap();
    assert_eq!(cache["latest"], "2.0.0");
    assert!(cache["checked_at"].as_u64().unwrap() > now_secs() - 60);
}

#[test]
fn missing_cache_fetches() {
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let dir = temp_cache_dir();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.expect("update should surface on first run");
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
    std::fs::write(dir.join("mytool.json"), "{{{ not json").unwrap();

    let result = check_with("mytool", "1.0.0", &dir, &server.url);

    let info = result.expect("corrupt cache must fall back to a live fetch");
    assert_eq!(info.latest, "2.0.0");
    assert_eq!(server.request_count(), 1);
}

#[test]
fn unwritable_cache_dir_still_returns_result_without_panicking() {
    // A file where a directory should be: cache read AND write both fail.
    let server = TestServer::spawn("HTTP/1.1 200 OK", crates_io_payload(&[("2.0.0", false)]));
    let blocker = temp_cache_dir().join("blocker");
    std::fs::write(&blocker, "not a dir").unwrap();

    let result = check_with("mytool", "1.0.0", &blocker, &server.url);

    let info = result.expect("cache write failure must not swallow the result");
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

#[test]
fn calendar_versions_compare_without_semver_assumptions() {
    // Calendar versioning: 2026.10.12 is newer than 2026.9.28.
    let server = TestServer::spawn(
        "HTTP/1.1 200 OK",
        crates_io_payload(&[("2026.10.12", false), ("2026.9.28", false)]),
    );
    let dir = temp_cache_dir();

    let result = check_with("mytool", "2026.9.28", &dir, &server.url);

    let info = result.expect("calendar-version update should surface");
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
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).unwrap()).unwrap();
    assert_eq!(cache["ttl_secs"].as_u64().unwrap(), TTL_SECS * 2);
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

    let info = result.expect("first stable version must be selected");
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
        serde_json::from_str(&std::fs::read_to_string(dir.join("mytool.json")).unwrap()).unwrap();
    assert_eq!(
        cache["latest"], "1.5.0",
        "backoff must preserve the known-good latest"
    );
    assert_eq!(
        cache["ttl_secs"].as_u64().unwrap(),
        24 * 60 * 60,
        "transport backoff uses the short (1 day) window"
    );

    // The preserved latest stays usable inside the backoff window: zero HTTP.
    let second = check_with("mytool", "1.0.0", &dir, &server.url);
    let info = second.expect("preserved latest must surface within backoff");
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

    let info = result.expect("future-dated cache must be refetched, not trusted");
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

    let info = result.expect("a 3s response must fit the total timeout budget");
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
    let exe = std::env::current_exe().expect("locate test binary");
    let output = std::process::Command::new(exe)
        .args(["--exact", "debug_signal_child", "--nocapture"])
        .env("GENESIS_UPDATE_CHECK_DEBUG", "1")
        .stderr(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .output()
        .expect("spawn child test");
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
