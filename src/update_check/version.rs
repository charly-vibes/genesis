//! Version classification and comparison for update checks.
//!
//! Purpose: decide whether a fetched latest version is strictly newer than
//! the installed one (genesis-u47). Responsibilities: stable-version
//! filtering and the two comparison schemes. Rationale: ranking is kept
//! scheme-agnostic and side-effect-free so cache reads stay pure.

/// A version is "stable" if it does not carry a semver pre-release identifier.
/// Versions that do not parse as semver (shouldn't happen on crates.io) are
/// treated as stable — ranking itself is scheme-agnostic ([`is_newer`]).
pub(super) fn is_stable(version: &str) -> bool {
    match semver::Version::parse(version) {
        Ok(v) => v.pre.is_empty(),
        Err(_) => true,
    }
}

/// Returns `true` when `latest` is strictly newer than `current` (genesis-u47).
///
/// Semver when both versions parse as such (pre-release ordering included);
/// otherwise a calendar/numeric comparison of dot-separated components
/// (`2026.9.28` < `2026.10.4`, missing trailing components rank as `0`).
/// Non-comparable pairs (non-numeric components) conservatively report "no
/// update" — a cached `latest` must never surface as a downgrade suggestion.
pub(super) fn is_newer(latest: &str, current: &str) -> bool {
    if let (Ok(latest_v), Ok(current_v)) = (
        semver::Version::parse(latest),
        semver::Version::parse(current),
    ) {
        return latest_v > current_v;
    }
    is_newer_calendar(latest, current)
}

/// Calendar/numeric fallback for versions that do not both parse as semver.
/// Compares dot-separated numeric components left to right; missing trailing
/// components rank as `0`; non-numeric components make the pair incomparable
/// (conservatively "no update"). A leading `v`/`V` (e.g. `v2026.10.4`) is
/// stripped before comparison so v-prefixed calendar versions still rank
/// (genesis-3i5).
fn is_newer_calendar(latest: &str, current: &str) -> bool {
    let mut latest_parts = strip_v(latest).split('.');
    let mut current_parts = strip_v(current).split('.');
    while let Some((ln, cn)) = next_components(&mut latest_parts, &mut current_parts) {
        if ln != cn {
            return ln > cn;
        }
    }
    false
}

/// Drop a leading `v`/`V` so v-prefixed calendar versions still rank
/// (genesis-3i5).
fn strip_v(s: &str) -> &str {
    s.strip_prefix(['v', 'V']).unwrap_or(s)
}

/// Next component pair as numbers. `None` when both sides are exhausted
/// (no newer component seen) or either component is non-numeric — not
/// rankable, stay silent.
fn next_components(
    latest: &mut std::str::Split<'_, char>,
    current: &mut std::str::Split<'_, char>,
) -> Option<(u64, u64)> {
    let (l, c) = (latest.next(), current.next());
    if l.is_none() && c.is_none() {
        return None;
    }
    let (Ok(ln), Ok(cn)) = (
        l.unwrap_or("0").parse::<u64>(),
        c.unwrap_or("0").parse::<u64>(),
    ) else {
        return None;
    };
    Some((ln, cn))
}
