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
//! - **Newest-wins versioning.** Notification only when the latest stable
//!   version is strictly newer than the installed one (genesis-u47): semver
//!   comparison when both parse, calendar/numeric dot-component comparison
//!   otherwise (e.g. `2026.9.28` < `2026.10.4`). Yanked and semver
//!   pre-release versions are filtered out.
//!
//! Internal layout: [`cache`] owns the cache-file format, [`version`] owns
//! comparison, [`http`] owns the bounded transport, and [`fetch`] owns the
//! fresh-cache/fetch/persist pipeline behind [`check_with`].

use std::path::PathBuf;
use std::time::Duration;

mod cache;
mod fetch;
mod http;
mod version;

use fetch::{fetch_latest, fresh_cache_hit, persist_outcome};

/// Default cache TTL: 7 days.
pub const DEFAULT_TTL_SECS: u64 = 7 * 24 * 60 * 60;

/// crates.io API base URL.
pub const CRATES_IO_API: &str = "https://crates.io/api/v1/crates";

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
/// `<cache>` honors `XDG_CACHE_HOME` as the cache root (XDG Base Directory
/// spec, genesis-39r), falling back to `$HOME/.cache`.
/// Returns `None` when neither is set (the caller should skip silently).
pub fn cache_path(crate_name: &str) -> Option<PathBuf> {
    if !is_valid_crate_name(crate_name) {
        return None;
    }
    let base = if let Some(xdg) = std::env::var("XDG_CACHE_HOME")
        .ok()
        .filter(|v| !v.is_empty())
    {
        PathBuf::from(xdg)
    } else {
        PathBuf::from(std::env::var("HOME").ok().filter(|v| !v.is_empty())?).join(".cache")
    };
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
    // 1. Fresh cache short-circuits: zero HTTP, zero latency.
    if let Some(hit) = fresh_cache_hit(&cache_file, crate_name, current_version) {
        return hit;
    }
    // 2./3. Fetch, persist, and map; backoff writes share the cache file this call reads.
    let outcome = fetch_latest(api_base, crate_name, &cache_file)?;
    persist_outcome(&cache_file, crate_name, current_version, outcome)
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
