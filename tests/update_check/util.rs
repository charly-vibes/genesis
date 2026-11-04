//! Shared local-server/cache fixtures for the `update_check` test binaries.
//! Compiled per-binary via `#[path]` so each binary stays under the
//! pretender file-lines budget.

#![allow(dead_code)]

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::common::TestUnwrap;

/// Minimal local HTTP server: answers every request with the canned response,
/// counts requests, and records the raw request bytes for header assertions.
pub struct TestServer {
    pub url: String,
    pub requests: Arc<AtomicUsize>,
    pub last_request: Arc<std::sync::Mutex<String>>,
}

impl TestServer {
    pub fn spawn(status_line: impl Into<String>, body: impl Into<String>) -> Self {
        let status_line = status_line.into();
        let body = body.into();
        Self::spawn_fn(move |_req| (status_line.clone(), body.clone()))
    }

    pub fn spawn_fn<F>(handler: F) -> Self
    where
        F: Fn(&str) -> (String, String) + Send + Sync + 'static,
    {
        let listener = TcpListener::bind("127.0.0.1:0").require("bind test server");
        let url = format!("http://{}", listener.local_addr().require("local_addr"));
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
                let body_text = String::from_utf8_lossy(&buf).to_string();
                reqs.fetch_add(1, Ordering::SeqCst);
                *value.lock().require("lock") = body_text.clone();
                let (status, body) = handler(&body_text);
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

    pub fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    pub fn last_request_contains(&self, needle: &str) -> bool {
        self.last_request.lock().require("lock").contains(needle)
    }

    pub fn last_request(&self) -> String {
        self.last_request.lock().require("lock").clone()
    }
}

pub fn crates_io_payload_same_timestamp(versions: &[(&str, bool)]) -> String {
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

pub fn crates_io_payload(versions: &[(&str, bool)]) -> String {
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

pub fn temp_cache_dir() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "genesis-update-check-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .require("value")
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).require("create cache dir");
    dir
}

pub fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .require("value")
        .as_secs()
}

pub const TTL_SECS: u64 = 7 * 24 * 60 * 60;

pub fn write_cache(
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
    .require("write cache");
}
