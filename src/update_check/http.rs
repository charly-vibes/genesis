//! HTTP transport for update checks.
//!
//! Purpose: fetch crates.io bodies within a bounded time budget.
//! Responsibilities: agent construction, GET dispatch, and backoff
//! bookkeeping on rate limits and transport errors. Rationale: every failure
//! is fail-silent (contract genesis-2ex) but leaves a backoff entry so an
//! unhappy network is retried at most once per window.

/// crates.io client identity sent as `User-Agent` (rate-limit politeness).
const USER_AGENT: &str = concat!("genesis-vibes/", env!("CARGO_PKG_VERSION"));

/// Backoff TTL applied after transport errors and corrupt response bodies
/// (e.g. offline, proxy garbage), so an unhappy network doesn't tax every
/// subsequent invocation.
pub(super) const ERROR_BACKOFF_SECS: u64 = 24 * 60 * 60;

use super::cache::{backoff_entry, write_cache};
use super::{CONNECT_TIMEOUT, DEFAULT_TTL_SECS, TOTAL_TIMEOUT, debug_emit};

/// GET `url` and return the body as a string; on HTTP-status or transport
/// errors, record the appropriate backoff entry in `backoff_cache_file` and
/// return `None` (terminal).
pub(super) fn http_get_body(
    url: &str,
    backoff_cache_file: &std::path::Path,
    user_agent: Option<&str>,
) -> Option<String> {
    let response = send_request(url, backoff_cache_file, user_agent)?;
    match response.into_string() {
        Ok(body) => Some(body),
        Err(_) => {
            debug_emit("unreadable response body");
            None
        }
    }
}

/// Send the GET request, mapping HTTP-status and transport errors to their
/// backoff entries and a terminal `None`.
fn send_request(
    url: &str,
    backoff_cache_file: &std::path::Path,
    user_agent: Option<&str>,
) -> Option<ureq::Response> {
    let agent = http_agent(user_agent);
    match agent.get(url).call() {
        Ok(response) => Some(response),
        Err(ureq::Error::Status(code, _)) => {
            backoff_for_status(url, code, backoff_cache_file);
            None
        }
        Err(err) => transport_error(backoff_cache_file, err),
    }
}

/// Transport (non-status) error: note it and take the short backoff.
fn transport_error(
    backoff_cache_file: &std::path::Path,
    err: ureq::Error,
) -> Option<ureq::Response> {
    debug_emit(&format!("transport error: {err}"));
    write_cache(
        backoff_cache_file,
        backoff_entry(backoff_cache_file, ERROR_BACKOFF_SECS),
    );
    None
}

fn http_agent(user_agent: Option<&str>) -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .user_agent(user_agent.unwrap_or(USER_AGENT))
        .build()
}

/// Record the backoff entry appropriate for an HTTP status: rate limits
/// (403/429) double the default TTL; everything else takes the short backoff.
fn backoff_for_status(url: &str, code: u16, backoff_cache_file: &std::path::Path) {
    let ttl_secs = if code == 403 || code == 429 {
        DEFAULT_TTL_SECS * 2
    } else {
        // 404 (wrong crate name!), 5xx, …: short backoff.
        ERROR_BACKOFF_SECS
    };
    debug_emit(&format!("HTTP {code} for {url}"));
    write_cache(
        backoff_cache_file,
        backoff_entry(backoff_cache_file, ttl_secs),
    );
}
