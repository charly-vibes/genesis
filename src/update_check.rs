//! Update availability checks against crates.io (feature `update-check`).
//!
//! Design contract (genesis-2ex):
//!
//! - **Binaries, not libs, notify.** Dependents call [`check`] with THEIR OWN
//!   crate name and installed version; genesis never checks itself from inside
//!   a dependent. This module only supplies the mechanism.
//! - **Cached-passive.** At most one fetch per cache TTL (default 7 days);
//!   results are cached at `<cache>/genesis/update-check/<crate>.json`.
//! - **Fail-silent.** Any error — network, timeout, corrupt cache, unwritable
//!   cache dir — degrades to `None`. This function must never panic, never
//!   block meaningfully (2s connect / 5s total), and never produce
//!   user-facing errors. Set `GENESIS_UPDATE_CHECK_DEBUG=1` to get one
//!   stderr line per skip/fail reason while wiring a dependent.
//! - **CI-aware.** `CI=true` or `GENESIS_NO_UPDATE_CHECK=<non-empty>` skips
//!   the check entirely, before any IO.
//! - **Rate-limit polite.** crates.io 403/429 responses double the cache TTL
//!   and send a descriptive `User-Agent`.
//! - **Scheme-agnostic versioning.** Comparison is `current != latest` with
//!   yanked and semver pre-release versions filtered out — calendar versions
//!   (e.g. `2026.9.28`) need no special-casing.

use std::path::PathBuf;
use std::time::Duration;

/// Default cache TTL: 7 days.
pub const DEFAULT_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// Backoff TTL applied after transport errors (e.g. offline), so an
/// unreachable network doesn't tax every subsequent invocation.
const ERROR_BACKOFF_SECS: u64 = 24 * 60 * 60;

/// crates.io API base URL.
pub const CRATES_IO_API: &str = "https://crates.io/api/v1/crates";

const USER_AGENT: &str = concat!("genesis-vibes/", env!("CARGO_PKG_VERSION"));
/// Connect timeout for crates.io requests.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(2);
/// Total request budget: connect + send + receive. Generous enough for slow
/// links (a 3s response must fit) while still bounded (genesis-4mq EDGE-003).
pub const TOTAL_TIMEOUT: Duration = Duration::from_secs(5);

/// An available update for a crate, as surfaced by [`check`] / [`check_with`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateInfo {
    /// The crate that was checked (the caller's own crate).
    pub crate_name: String,
    /// Latest published stable version.
    pub latest: String,
    /// Version the caller reported as installed.
    pub current: String,
    /// Publication timestamp of the latest version (RFC 3339), when known.
    pub published_at: Option<String>,
}

/// One-line, actionable update notice.
///
/// Example: `mytool 1.2.3 available — you have 1.2.2 (cargo install mytool)`
pub fn notice(info: &UpdateInfo) -> String {
    format!(
        "{} {} available — you have {} (cargo install {})",
        info.crate_name, info.latest, info.current, info.crate_name
    )
}

/// Resolve the cache file path for a crate: `<cache>/genesis/update-check/<crate>.json`
///
/// `<cache>` honors `XDG_CACHE_HOME`, falling back to `$HOME/.cache`.
/// Returns `None` when neither is set (the caller should skip silently).
pub fn cache_path(crate_name: &str) -> Option<PathBuf> {
    if !is_valid_crate_name(crate_name) {
        return None;
    }
    let base = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|v| !v.is_empty())
        .or_else(|| std::env::var("HOME").ok().filter(|v| !v.is_empty()))
        .map(|home| PathBuf::from(home).join(".cache"))?;
    Some(
        base.join("genesis")
            .join("update-check")
            .join(format!("{crate_name}.json")),
    )
}

/// Check whether a newer stable version of `crate_name` exists on crates.io.
///
/// Skips entirely (returns `None`, no IO) when `CI=true` or
/// `GENESIS_NO_UPDATE_CHECK` is set to a non-empty value. Otherwise behaves
/// exactly like [`check_with`] against [`CRATES_IO_API`] with the default
/// cache location.
///
/// `current_version` is the caller's own installed version
/// (`env!("CARGO_PKG_VERSION")` for a binary wiring its own crate).
pub fn check(crate_name: &str, current_version: &str) -> Option<UpdateInfo> {
    if std::env::var("GENESIS_NO_UPDATE_CHECK")
        .ok()
        .is_some_and(|v| !v.is_empty())
    {
        return None;
    }
    let ci = std::env::var("CI")
        .ok()
        .is_some_and(|v| v == "true" || v == "1");
    if ci {
        return None;
    }
    let cache_dir = cache_path(crate_name)?.parent()?.to_path_buf();
    check_with(crate_name, current_version, &cache_dir, CRATES_IO_API)
}

/// Like [`check`], but with explicit cache directory and API base URL.
///
/// Hidden from docs: this is the hermetic-test and advanced-wiring entry
/// point. It performs NO env-var skipping — callers manage that themselves
/// via [`check`].
#[doc(hidden)]
pub fn check_with(
    crate_name: &str,
    current_version: &str,
    cache_dir: &std::path::Path,
    api_base: &str,
) -> Option<UpdateInfo> {
    if !is_valid_crate_name(crate_name) {
        debug_emit("rejecting invalid crate name");
        return None;
    }
    let cache_file = cache_dir.join(format!("{crate_name}.json"));

    // 1. Fresh cache short-circuits: zero HTTP, zero latency. A future
    //    checked_at (clock skew) counts as stale, never maximally fresh.
    if let Some(cached) = read_cache(&cache_file) {
        let now = unix_now();
        if now >= cached.checked_at && now - cached.checked_at < cached.ttl_secs {
            return cached.latest.and_then(|latest| {
                (latest != current_version).then(|| UpdateInfo {
                    crate_name: crate_name.to_string(),
                    latest,
                    current: current_version.to_string(),
                    published_at: cached.published_at,
                })
            });
        }
    }

    // 2. Fetch from the API. Backoff writes go to the same cache file this
    //    call reads from (never the default cache location).
    let outcome = fetch_latest(api_base, crate_name, &cache_file)?;

    // 3. Persist and map. Comparison happens at read time (latest is cached
    //    even when equal to current, so "up to date" also honors the TTL).
    match outcome.latest {
        Some(latest) => {
            write_cache(
                &cache_file,
                CacheEntry {
                    checked_at: unix_now(),
                    latest: Some(latest.clone()),
                    published_at: outcome.published_at.clone(),
                    ttl_secs: DEFAULT_TTL_SECS,
                },
            );
            (latest != current_version).then(|| UpdateInfo {
                crate_name: crate_name.to_string(),
                latest,
                current: current_version.to_string(),
                published_at: outcome.published_at,
            })
        }
        None => {
            // No usable stable version (all yanked/prerelease): cache the miss.
            write_cache(
                &cache_file,
                CacheEntry {
                    checked_at: unix_now(),
                    latest: None,
                    published_at: None,
                    ttl_secs: DEFAULT_TTL_SECS,
                },
            );
            None
        }
    }
}

struct FetchOutcome {
    latest: Option<String>,
    published_at: Option<String>,
}

/// Fetch and select the latest stable version. Side effect: on rate-limit (403/429)
/// or transport errors, writes a backoff cache entry (to `backoff_cache_file`) so
/// the next call within the backoff window short-circuits. Returns `None`
/// (terminal) on unrecoverable errors.
fn fetch_latest(
    api_base: &str,
    crate_name: &str,
    backoff_cache_file: &std::path::Path,
) -> Option<FetchOutcome> {
    let agent: ureq::Agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .user_agent(USER_AGENT)
        .build();
    let url = format!("{api_base}/{crate_name}");

    let body = match agent.get(&url).call() {
        Ok(resp) => match resp.into_string() {
            Ok(b) => b,
            Err(_) => {
                debug_emit("unreadable response body");
                return None;
            }
        },
        Err(ureq::Error::Status(code, _)) => {
            let ttl_secs = if code == 403 || code == 429 {
                // Rate-limited: double the default TTL.
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
            return None;
        }
        Err(err) => {
            // Transport error (offline, timeout, refused): short backoff.
            debug_emit(&format!("transport error: {err}"));
            write_cache(
                backoff_cache_file,
                backoff_entry(backoff_cache_file, ERROR_BACKOFF_SECS),
            );
            return None;
        }
    };

    let parsed: CratesIoResponse = match serde_json::from_str(&body) {
        Ok(p) => p,
        Err(err) => {
            debug_emit(&format!("malformed response body: {err}"));
            return None;
        }
    };
    // crates.io returns versions newest-first, so the first stable entry IS
    // the latest — never compare created_at strings (ties would resolve to
    // the oldest version). genesis-4mq CORR-001.
    let latest = parsed
        .versions
        .iter()
        .find(|v| !v.yanked && is_stable(&v.num));
    let Some(latest) = latest else {
        debug_emit("no non-yanked stable version published");
        return None;
    };

    Some(FetchOutcome {
        latest: Some(latest.num.clone()),
        published_at: Some(latest.created_at.clone()),
    })
}

/// A version is "stable" if it does not carry a semver pre-release identifier.
/// Versions that do not parse as semver (shouldn't happen on crates.io) are
/// treated as stable — comparison itself is scheme-agnostic (`current != latest`).
fn is_stable(version: &str) -> bool {
    match semver::Version::parse(version) {
        Ok(v) => v.pre.is_empty(),
        Err(_) => true,
    }
}

#[derive(Debug, serde::Deserialize)]
struct CratesIoResponse {
    #[serde(default)]
    versions: Vec<CratesIoVersion>,
}

/// Backoff cache entry for a failed fetch. A previously known-good
/// `latest`/`published_at` is preserved (genesis-4mq CORR-003) so a failed
/// fetch never downgrades cached knowledge to "no update".
fn backoff_entry(backoff_cache_file: &std::path::Path, ttl_secs: u64) -> CacheEntry {
    let prior = read_cache(backoff_cache_file);
    CacheEntry {
        checked_at: unix_now(),
        latest: prior.as_ref().and_then(|e| e.latest.clone()),
        published_at: prior.as_ref().and_then(|e| e.published_at.clone()),
        ttl_secs,
    }
}

/// A crate name is safe to interpolate into URLs and cache filenames:
/// non-empty, ASCII alphanumeric plus `-` and `_` (genesis-4mq CORR-002).
fn is_valid_crate_name(crate_name: &str) -> bool {
    !crate_name.is_empty()
        && crate_name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// One stderr line when `GENESIS_UPDATE_CHECK_DEBUG` is set to a non-empty
/// value. De-risks silent 404s while wiring dependents (wai's crate is
/// `wai-cli`, NOT `wai`). genesis-4mq EXCL-002.
fn debug_emit(reason: &str) {
    if std::env::var("GENESIS_UPDATE_CHECK_DEBUG")
        .ok()
        .is_some_and(|v| !v.is_empty())
    {
        eprintln!("genesis-vibes update-check: {reason}");
    }
}

#[derive(Debug, serde::Deserialize)]
struct CratesIoVersion {
    num: String,
    #[serde(default)]
    yanked: bool,
    #[serde(default)]
    created_at: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CacheEntry {
    checked_at: u64,
    latest: Option<String>,
    published_at: Option<String>,
    ttl_secs: u64,
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn read_cache(path: &std::path::Path) -> Option<CacheEntry> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

/// Best-effort atomic cache write (temp file + rename). All failures —
/// missing dir, read-only fs, path is a file — are swallowed by contract.
fn write_cache(path: &std::path::Path, entry: CacheEntry) {
    let Some(dir) = path.parent() else { return };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let Ok(json) = serde_json::to_string(&entry) else {
        return;
    };
    let Ok(tmp) = tempfile::Builder::new()
        .prefix(".genesis-update-check-")
        .tempfile_in(dir)
    else {
        return;
    };
    if std::fs::write(tmp.path(), json).is_err() {
        return;
    }
    // rename replaces the target on all platforms tempfile supports.
    let _ = tmp.persist(path);
}
