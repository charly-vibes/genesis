//! Update-check cache layer: entry format, TTL/backoff records, and the
//! process-clock helper.

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct CacheEntry {
    pub(crate) checked_at: u64,
    pub(crate) latest: Option<String>,
    pub(crate) published_at: Option<String>,
    pub(crate) ttl_secs: u64,
}

/// Backoff cache entry for a failed fetch. A previously known-good
/// `latest`/`published_at` is preserved (genesis-4mq CORR-003) so a failed
/// fetch never downgrades cached knowledge to "no update".
pub(super) fn backoff_entry(backoff_cache_file: &std::path::Path, ttl_secs: u64) -> CacheEntry {
    let prior = read_cache(backoff_cache_file);
    CacheEntry {
        checked_at: unix_now(),
        latest: prior.as_ref().and_then(|e| e.latest.clone()),
        published_at: prior.as_ref().and_then(|e| e.published_at.clone()),
        ttl_secs,
    }
}

pub(super) fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(super) fn read_cache(path: &std::path::Path) -> Option<CacheEntry> {
    let contents = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Best-effort atomic cache write (temp file + rename). All failures —
/// missing dir, read-only fs, path is a file — are swallowed by contract.
pub(super) fn write_cache(path: &std::path::Path, entry: CacheEntry) {
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
