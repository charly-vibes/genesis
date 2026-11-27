//! Cache-path and timestamp resolution tests for `feedback::scratch`
//! (genesis-4xl): XDG fallthrough, `now_iso` format, and the XDG-honoring
//! scratch dir.

use super::super::now_iso;
use super::*;

/// genesis-4xl: an empty (or unset) `XDG_CACHE_HOME` must fall through to
/// `$HOME/.cache` or the temp dir — never resolve to a relative path that
/// writes `errors.jsonl` into the cwd. Pure-function variant, so no
/// process-env mutation races the parallel scratch tests.
#[test]
fn test_cache_base_empty_xdg_falls_through() {
    use super::super::cache_base_for;

    let base = cache_base_for(Some(String::new()), Some("/home/u".into()));
    assert_eq!(base, std::path::PathBuf::from("/home/u/.cache"));

    // Empty XDG + no HOME → temp dir (absolute).
    let base = cache_base_for(Some(String::new()), None);
    assert_eq!(base, std::env::temp_dir());
    assert!(base.is_absolute(), "must not be relative: {base:?}");

    // Empty HOME falls through too.
    let base = cache_base_for(None, Some(String::new()));
    assert_eq!(base, std::env::temp_dir());

    // A set, non-empty XDG still wins.
    let base = cache_base_for(Some("/custom/cache".into()), Some("/home/u".into()));
    assert_eq!(base, std::path::PathBuf::from("/custom/cache"));
}

#[test]
fn test_timestamp_format() {
    let ts = now_iso();
    // ISO 8601 format: YYYY-MM-DDTHH:MM:SSZ
    assert_eq!(ts.len(), 20, "expected ISO 8601 length, got: {}", ts);
    assert!(ts.ends_with('Z'), "expected UTC timezone marker");
    assert_eq!(&ts[4..5], "-", "expected dash after year");
    assert_eq!(&ts[7..8], "-", "expected dash after month");
    assert_eq!(&ts[10..11], "T", "expected T separator");
}

#[test]
fn test_scratch_dir_uses_xdg() {
    let prev = std::env::var("XDG_CACHE_HOME").ok();
    unsafe { std::env::set_var("XDG_CACHE_HOME", "/custom/cache") };
    let dir = scratch_dir("test-tool");
    assert_eq!(dir, std::path::PathBuf::from("/custom/cache/test-tool"));
    match prev {
        Some(v) => unsafe { std::env::set_var("XDG_CACHE_HOME", v) },
        None => unsafe { std::env::remove_var("XDG_CACHE_HOME") },
    }
}
