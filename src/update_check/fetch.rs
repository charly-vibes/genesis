//! Fetch pipeline for update checks.
//!
//! Purpose: turn a crates.io API response (or a fresh cache hit) into an
//! [`UpdateInfo`]. Responsibilities: response modeling, latest-stable
//! selection, and the persist-and-map step. Rationale: version knowledge is
//! cached even when equal to current, so "up to date" also honors the TTL
//! (genesis-2ex), and failures never downgrade cached knowledge.

use super::cache::{CacheEntry, backoff_entry, read_cache, unix_now, write_cache};
use super::http::{self, http_get_body};
use super::version::{is_newer, is_stable};
use super::{DEFAULT_TTL_SECS, UpdateInfo, debug_emit};

pub(super) struct FetchOutcome {
    latest: Option<String>,
    published_at: Option<String>,
}

/// A fresh-cache hit mapped to an update, if the cached latest is newer.
/// A future `checked_at` (clock skew) counts as stale, never maximally fresh.
pub(super) fn fresh_cache_hit(
    cache_file: &std::path::Path,
    crate_name: &str,
    current_version: &str,
) -> Option<Option<UpdateInfo>> {
    let cached = read_cache(cache_file)?;
    let now = unix_now();
    let fresh = now >= cached.checked_at && now - cached.checked_at < cached.ttl_secs;
    if !fresh {
        return None;
    }
    Some(cached.latest.and_then(|latest| {
        to_update_info(crate_name, current_version, latest, cached.published_at)
    }))
}

/// Persist the fetch outcome under the default TTL, mapping it to an update
/// when the fetched latest is newer than the installed version.
///
/// Backoff writes share the cache file the caller read, so a stale
/// `cache_file` path must not be reused across check rounds.
pub(super) fn persist_outcome(
    cache_file: &std::path::Path,
    crate_name: &str,
    current_version: &str,
    outcome: FetchOutcome,
) -> Option<UpdateInfo> {
    let (latest, published_at) = (outcome.latest, outcome.published_at);
    write_cache(
        cache_file,
        CacheEntry {
            checked_at: unix_now(),
            latest: latest.clone(),
            published_at: published_at.clone(),
            ttl_secs: DEFAULT_TTL_SECS,
        },
    );
    to_update_info(crate_name, current_version, latest?, published_at)
}

/// Map a candidate latest version to an [`UpdateInfo`] when it is strictly
/// newer than the installed one; `None` latest (no version knowledge) maps
/// to no update.
fn to_update_info(
    crate_name: &str,
    current_version: &str,
    latest: String,
    published_at: Option<String>,
) -> Option<UpdateInfo> {
    is_newer(&latest, current_version).then(|| UpdateInfo {
        crate_name: crate_name.to_string(),
        latest,
        current: current_version.to_string(),
        published_at,
    })
}

/// Fetch and select the latest stable version. Side effect: on rate-limit (403/429),
/// transport errors, corrupt response bodies, or an empty registry (no non-yanked
/// stable version), writes a backoff cache entry (to `backoff_cache_file`) so the
/// next call within the window short-circuits (genesis-3i5): a corrupt response
/// takes the short transport-error backoff, an empty registry the full cache TTL
/// (there is genuinely nothing newer to learn until versions change). Returns
/// `None` (terminal) on unrecoverable errors.
pub(super) fn fetch_latest(
    api_base: &str,
    crate_name: &str,
    backoff_cache_file: &std::path::Path,
    user_agent: Option<&str>,
) -> Option<FetchOutcome> {
    let url = format!("{api_base}/{crate_name}");
    let body = http_get_body(&url, backoff_cache_file, user_agent)?;
    let parsed = parse_registry(&body, backoff_cache_file)?;
    match select_latest_stable(parsed) {
        Some(outcome) => Some(outcome),
        None => {
            write_cache(
                backoff_cache_file,
                backoff_entry(backoff_cache_file, DEFAULT_TTL_SECS),
            );
            None
        }
    }
}

/// Parse the registry body. A corrupt body takes the short transport-
/// error backoff (genesis-3i5) and is terminal.
fn parse_registry(body: &str, backoff_cache_file: &std::path::Path) -> Option<CratesIoResponse> {
    match serde_json::from_str(body) {
        Ok(p) => Some(p),
        Err(err) => {
            debug_emit(&format!("malformed response body: {err}"));
            write_cache(
                backoff_cache_file,
                backoff_entry(backoff_cache_file, http::ERROR_BACKOFF_SECS),
            );
            None
        }
    }
}

/// Pick the first non-yanked stable version (newest-first ordering).
///
/// crates.io returns versions newest-first, so the first stable entry IS the
/// latest — never compare created_at strings (ties would resolve to the
/// oldest version). genesis-4mq CORR-001.
fn select_latest_stable(parsed: CratesIoResponse) -> Option<FetchOutcome> {
    let Some(latest) = parsed
        .versions
        .iter()
        .find(|v| !v.yanked && is_stable(&v.num))
    else {
        debug_emit("no non-yanked stable version published");
        return None;
    };
    Some(FetchOutcome {
        latest: Some(latest.num.clone()),
        published_at: Some(latest.created_at.clone()),
    })
}

#[derive(Debug, serde::Deserialize)]
struct CratesIoResponse {
    #[serde(default)]
    versions: Vec<CratesIoVersion>,
}

#[derive(Debug, serde::Deserialize)]
struct CratesIoVersion {
    num: String,
    #[serde(default)]
    yanked: bool,
    #[serde(default)]
    created_at: String,
}
