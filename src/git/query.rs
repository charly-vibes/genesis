//! Read-only repository queries: tracked/ignored state, content hashes,
//! and per-path status.

use std::path::Path;

use super::GitError;
use super::changes::relative_to;
use super::runner::{EnvPolicy, run};

/// The single-path status of a worktree/index path, derived from the
/// `XY` code of `git status --porcelain -- <path>` (see [`path_status`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStatus {
    /// The index differs from `HEAD` (non-blank `X` column) after the
    /// worktree check below — i.e. staged-only change. Precedence:
    /// when both columns are set (e.g. `MM`), [`PathStatus::Dirty`] wins
    /// because the worktree state is the later snapshot.
    Staged,
    /// The worktree differs from the index (non-blank `Y` column), or the
    /// path is unmerged (`U` in either column, or `DD`/`AA`) — unmerged
    /// states map to dirty by contract because both mean "this path needs
    /// attention before it can be committed".
    Dirty,
    /// The path is untracked (`??`).
    Untracked,
}

/// Status of exactly one path via `git status --porcelain -- <path>`.
///
/// Returns `Ok(None)` when the path is clean (empty porcelain body —
/// which also covers paths unknown to git), `Some(_)` otherwise:
/// `??` → [`PathStatus::Untracked`]; unmerged (`U` in either column,
/// `DD`, `AA`) → [`PathStatus::Dirty`] (documented mapping); non-blank
/// `Y` (worktree differs from index) → [`PathStatus::Dirty`]; non-blank
/// `X` (index differs from `HEAD`) → [`PathStatus::Staged`]. Exactly one
/// path is queried because the dispatcher contract (hook-time decisions
/// on a single file) needs one unambiguous answer, not a list. Git
/// failures surface as typed errors.
pub fn path_status(root: &Path, rel: &Path) -> Result<Option<PathStatus>, GitError> {
    path_status_with_policy(root, rel, EnvPolicy::Inherit)
}

/// [`path_status`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn path_status_with_policy(
    root: &Path,
    rel: &Path,
    policy: EnvPolicy,
) -> Result<Option<PathStatus>, GitError> {
    let relative = relative_to(root, rel);
    let out = run(root, policy, &["status", "--porcelain", "--", &relative])?;
    let body = String::from_utf8_lossy(&out.stdout);
    // A single-path status body carries at most one record.
    let Some(line) = body.lines().next() else {
        return Ok(None);
    };
    Ok(status_from_columns(line.as_bytes()))
}

/// Classify a porcelain status row by its two XY columns. `None` for
/// malformed rows; both columns blank cannot appear for a listed path and
/// are treated as clean for safety.
fn status_from_columns(bytes: &[u8]) -> Option<PathStatus> {
    if bytes.len() < 2 {
        return None;
    }
    let (x, y) = (bytes[0], bytes[1]);
    Some(if x == b'?' || y == b'?' {
        PathStatus::Untracked
    } else if is_unmerged(x, y) || y != b' ' {
        // Unmerged maps to dirty (documented): both mean the path cannot
        // be committed as-is.
        PathStatus::Dirty
    } else if x != b' ' {
        PathStatus::Staged
    } else {
        return None;
    })
}

/// Unmerged XY combinations: `UU`, `DD`, `AA` (and the mixed forms).
fn is_unmerged(x: u8, y: u8) -> bool {
    x == b'U' || y == b'U' || (x == b'D' && y == b'D') || (x == b'A' && y == b'A')
}

/// Resolve the committed content of a path via `git rev-parse
/// HEAD:<path>` — the blob hash of the file as committed at HEAD.
pub fn committed_content_hash(root: &Path, path: &Path) -> Result<String, GitError> {
    committed_content_hash_with_policy(root, path, EnvPolicy::Inherit)
}

/// [`committed_content_hash`] with an explicit environment policy
/// (policy-aware variant; see [`run`] for what the policy controls).
pub fn committed_content_hash_with_policy(
    root: &Path,
    path: &Path,
    policy: EnvPolicy,
) -> Result<String, GitError> {
    let spec = format!("HEAD:{}", relative_to(root, path));
    let out = run(root, policy, &["rev-parse", "--verify", &spec])?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Whether the repository knows a path (via `git ls-files`), independent
/// of working-tree modifications. `Ok(false)` for paths git does not
/// know; a typed error for any other failure.
pub fn tracked(root: &Path, path: &Path) -> Result<bool, GitError> {
    tracked_with_policy(root, path, EnvPolicy::Inherit)
}

/// [`tracked`] with an explicit environment policy (policy-aware variant;
/// see [`run`] for what the policy controls).
pub fn tracked_with_policy(root: &Path, path: &Path, policy: EnvPolicy) -> Result<bool, GitError> {
    let relative = relative_to(root, path);
    match run(
        root,
        policy,
        &["ls-files", "--error-unmatch", "--", &relative],
    ) {
        Ok(_) => Ok(true),
        // exit 1: pathspec matched nothing known to git.
        Err(GitError::Git { code: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

/// Whether git ignores a single path (via `check-ignore -q`), as a
/// tri-state: `Ok(true)` = ignored (exit 0), `Ok(false)` = not ignored
/// (exit 1), `Err` = git failure (spawn failure or exit ≥ 128). Exactly
/// one path is accepted because `check-ignore` exits 0 when *any* of
/// several paths is ignored, which cannot express a per-path tri-state.
pub fn is_ignored(root: &Path, path: &Path) -> Result<bool, GitError> {
    is_ignored_with_policy(root, path, EnvPolicy::Inherit)
}

/// [`is_ignored`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn is_ignored_with_policy(
    root: &Path,
    path: &Path,
    policy: EnvPolicy,
) -> Result<bool, GitError> {
    let relative = relative_to(root, path);
    match run(root, policy, &["check-ignore", "-q", "--", &relative]) {
        Ok(_) => Ok(true),
        // exit 1: the path exists but is not ignored.
        Err(GitError::Git { code: 1, .. }) => Ok(false),
        // exit ≥ 128 (e.g. 128 outside a repository) and spawn failures
        // are git failures, not "not ignored".
        Err(other) => Err(other),
    }
}

/// Hash a worktree file's content via `git hash-object` (deterministic
/// for the same content).
pub fn content_hash(root: &Path, path: &Path) -> Result<String, GitError> {
    content_hash_with_policy(root, path, EnvPolicy::Inherit)
}

/// [`content_hash`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn content_hash_with_policy(
    root: &Path,
    path: &Path,
    policy: EnvPolicy,
) -> Result<String, GitError> {
    let relative = relative_to(root, path);
    let out = run(root, policy, &["hash-object", "--", &relative])?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
