//! Atomic file replacement and JSON helpers, shared across the fleet.
//!
//! ## Responsibilities
//!
//! - Replace a file in one observable step: stage uniquely-named sibling temp
//!   file → flush to the device → rename over the target
//! - Retry transient Windows sharing violations on the rename, bounded
//! - Typed JSON read/write helpers over the atomic path
//!
//! ## Rationale
//!
//! Ported from the strongest fleet implementation
//! (`dulce-de-leche/src/dot_ddl/persist.rs`, `stage_manifest` +
//! `with_sharing_retry`) rather than re-invented, per the add-fs-module
//! proposal. POSIX rename is atomic, so readers observe either the complete
//! old or the complete new file — never partial content — and a failure
//! anywhere before the rename leaves the prior target byte-for-byte intact.
//! On Windows `std::fs::rename` uses `MOVEFILE_REPLACE_EXISTING` and can fail
//! with a sharing violation while another process holds the target open; the
//! bounded retry (`with_sharing_retry`) rides those out.
//!
//! The staging file is opened no-follow (ddL's SEC-05 hardening): a link
//! swapped in at the staging path cannot redirect the staged write.
//!
//! No new runtime dependency: `std::fs` plus `serde_json` for the JSON
//! helpers only.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Errors surfaced by `genesis::fs` operations.
///
/// Typed so call sites can distinguish a failed read (missing file, io) from
/// malformed content (JSON) without string matching.
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    /// An underlying filesystem operation failed.
    #[error("fs: {0}")]
    Io(#[from] std::io::Error),
    /// The file exists but its content is not valid JSON for the target type.
    #[error("fs: invalid JSON in {}: {source}", path.display())]
    Json {
        /// The file that failed to parse.
        path: PathBuf,
        /// The serde error behind the failure.
        #[source]
        source: serde_json::Error,
    },
    /// The target path has no parent directory to stage the staging file in.
    #[error("fs: path has no parent directory: {0}")]
    NoParent(PathBuf),
}

/// Process-shared counter for staging names (task 1.4).
///
/// Shared across every staging call in the process, so two sequential writes
/// — even to different targets — never reuse a name. Combined with the pid,
/// two *processes* cannot collide either (ddL's counter was per-process only).
static STAGING_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Unique staging-file path for `target`: `.<name>.<pid>.<counter>.tmp`
/// in the target's own directory.
///
/// The leading dot and `.tmp` suffix make a stale staging file recognizable
/// as debris of this target (swept by a later cleanup, never required for a
/// successful write); the embedded pid and process-shared counter make the
/// name unique per writer.
pub fn staging_path_for(target: &Path) -> PathBuf {
    let dir = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| "file".into());
    let counter = STAGING_COUNTER.fetch_add(1, Ordering::Relaxed);
    dir.join(format!(
        ".{}.{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id(),
        counter
    ))
}

/// Replace `target` with `bytes` atomically.
///
/// Stages the bytes in a uniquely-named sibling temporary file in the target's
/// own directory (so the rename stays on one filesystem), opens it no-follow,
/// flushes it to the device, then renames it over the target. On any failure
/// before a successful rename, the prior target is left byte-for-byte intact
/// and the staging file is removed.
///
/// The rename step retries transient Windows sharing violations
/// ([`with_sharing_retry`]); on Unix the first attempt always runs. The
/// staging handle is closed before the rename, since on Windows a rename over
/// a file we ourselves hold open is a self-inflicted sharing violation.
pub fn atomic_write(target: &Path, bytes: &[u8]) -> Result<(), FsError> {
    if target.parent().is_none() {
        return Err(FsError::NoParent(target.to_path_buf()));
    }
    let staging = staging_path_for(target);

    let file = open_staging_no_follow(&staging).map_err(|e| {
        let _ = std::fs::remove_file(&staging);
        FsError::Io(e)
    })?;
    write_and_sync_staging(&file, bytes)?;
    drop(file);
    finalize_rename(&staging, target)
}

/// Write `bytes` to the staged file and fsync it, so the rename cannot
/// publish zero-length or torn contents after a crash.
fn write_and_sync_staging(file: &std::fs::File, bytes: &[u8]) -> Result<(), FsError> {
    let mut file = file;
    file.write_all(bytes).map_err(FsError::Io)?;
    file.sync_all().map_err(FsError::Io)
}

/// Rename the staged file over `target`, retrying transient Windows sharing
/// violations.
///
/// On failure the staging file is removed so no debris is left behind; on
/// success the parent directory is fsynced (best-effort) so the rename itself
/// survives a power loss.
fn finalize_rename(staging: &Path, target: &Path) -> Result<(), FsError> {
    let rename_result =
        with_sharing_retry(|| std::fs::rename(staging, target).map_err(FsError::Io));
    match rename_result {
        Ok(()) => {
            sync_parent_dir(target);
            Ok(())
        }
        Err(e) => {
            let _ = std::fs::remove_file(staging);
            Err(e)
        }
    }
}

/// Open the staging file for writing without following a link at the final
/// path component (ddL's SEC-05 hardening, ported).
///
/// `O_NOFOLLOW` (Unix) resp. `FILE_FLAG_OPEN_REPARSE_POINT` (Windows) makes
/// the refusal happen inside the open itself, so a link swapped in at the
/// staging path cannot redirect the staged write, and the check cannot be
/// raced past.
fn open_staging_no_follow(path: &Path) -> std::io::Result<std::fs::File> {
    #[cfg(unix)]
    {
        open_staging_unix(path)
    }
    #[cfg(windows)]
    {
        open_staging_windows(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        open_staging_other(path)
    }
}

/// Unix no-follow open: `O_NOFOLLOW` is platform-specific, so only define it
/// where the constant is known; other Unix flavors degrade to a plain open.
#[cfg(unix)]
fn open_staging_unix(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(target_os = "linux")]
    const O_NOFOLLOW: i32 = 0o400_000;
    #[cfg(target_os = "macos")]
    const O_NOFOLLOW: i32 = 0o100;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true).write(true).truncate(true);
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    opts.custom_flags(O_NOFOLLOW);
    opts.open(path)
}

/// Windows no-follow open: open the reparse point itself instead of following
/// it to the target.
#[cfg(windows)]
fn open_staging_windows(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    let mut opts = std::fs::OpenOptions::new();
    opts.create(true)
        .write(true)
        .truncate(true)
        .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT);
    opts.open(path)
}

/// Fallback plain open on platforms with no no-follow flag.
#[cfg(not(any(unix, windows)))]
fn open_staging_other(path: &Path) -> std::io::Result<std::fs::File> {
    std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
}

/// Windows sharing violation (`ERROR_SHARING_VIOLATION`) or lock violation
/// (`ERROR_LOCK_VIOLATION`) — the transient rename failures the retry is for.
fn is_sharing_violation(e: &FsError) -> bool {
    matches!(
        e,
        FsError::Io(io) if matches!(io.raw_os_error(), Some(32) | Some(33))
    )
}

/// Run `op`, retrying transient Windows file locks.
///
/// Windows LockFileEx locks held by antivirus scanners or other processes
/// block our rename step; exponential backoff (6 attempts, ~6.3 s worst case)
/// rides those out without slowing down real failures. Bounded: a process
/// holding the target open cannot turn the rename into a hang.
///
/// On Unix the first attempt runs and any error is returned immediately —
/// sharing violations cannot occur there, so the retry is inert.
pub fn with_sharing_retry<F>(op: F) -> Result<(), FsError>
where
    F: FnMut() -> Result<(), FsError>,
{
    const MAX_ATTEMPTS: u32 = 6;
    with_sharing_retry_bounded(MAX_ATTEMPTS, Duration::from_millis(100), op)
}

/// [`with_sharing_retry`] with an explicit bound: `max_attempts` attempts and
/// an exponential backoff starting at `base_delay` (doubling each retry).
///
/// Exposed so tests can exercise the retry policy with a fast, injected
/// failing operation instead of a real sharing violation (which only occurs
/// on Windows).
pub fn with_sharing_retry_bounded<F>(
    max_attempts: u32,
    base_delay: Duration,
    mut op: F,
) -> Result<(), FsError>
where
    F: FnMut() -> Result<(), FsError>,
{
    for attempt in 1..=max_attempts.max(1) {
        match op() {
            Ok(()) => return Ok(()),
            Err(e) if is_sharing_violation(&e) && attempt < max_attempts.max(1) => {
                std::thread::sleep(base_delay * 2u32.pow(attempt - 1));
            }
            Err(e) => return Err(e),
        }
    }
    unreachable!("retry loop always returns within the loop")
}

/// Best-effort fsync of the directory containing `path` (Unix only), so the
/// rename itself survives a power loss. Best-effort: a failed directory fsync
/// degrades durability but never loses the on-disk new state.
#[cfg(unix)]
fn sync_parent_dir(path: &Path) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::File::open(dir).map(|f| f.sync_all());
    }
}

/// Best-effort directory fsync is a Unix durability nicety; Windows relies on
/// MOVEFILE_REPLACE_EXISTING journaling semantics.
#[cfg(not(unix))]
fn sync_parent_dir(_path: &Path) {}

/// Deserialize a JSON file into `T`.
///
/// Malformed content yields [`FsError::Json`]; a missing or unreadable file
/// yields [`FsError::Io`]. Never panics.
pub fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, FsError> {
    let content = std::fs::read_to_string(path)?;
    serde_json::from_str(&content).map_err(|source| FsError::Json {
        path: path.to_path_buf(),
        source,
    })
}

/// Serialize `value` as pretty JSON and write it through [`atomic_write`]
/// (stage → flush → rename), so readers see either the complete old or the
/// complete new file.
pub fn atomic_write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), FsError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| FsError::Json {
        path: path.to_path_buf(),
        source,
    })?;
    atomic_write(path, &bytes)
}
