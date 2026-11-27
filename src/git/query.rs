//! Read-only repository queries: tracked/ignored state, content hashes,
//! and per-path status.

use std::path::Path;

use super::GitError;
use super::changes::relative_to;
use super::runner::{EnvPolicy, GitOutput, run, run_with_stdin};

/// Build a single-path pathspec for `git ls-files`/`status`/`check-ignore`
/// arguments: `:(literal)` magic disables glob interpretation so filenames
/// like `a[1].txt` match themselves, and `path == root` (empty relative)
/// maps to `.` so `ls-files --error-unmatch` does not exit 128.
fn literal_pathspec(relative: &str) -> String {
    let rel = if relative.is_empty() { "." } else { relative };
    format!(":(literal){rel}")
}

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
    let spec = literal_pathspec(&relative);
    let out = run(root, policy, &["status", "--porcelain", "--", &spec])?;
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
///
/// Unlike [`changed_files`](super::changes::changed_files), which
/// special-cases an unborn HEAD, a repository without commits yields a
/// typed [`GitError::Git`] (exit 128) here — there is no committed
/// content to resolve.
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
    let spec = literal_pathspec(&relative);
    match run(root, policy, &["ls-files", "--error-unmatch", "--", &spec]) {
        Ok(_) => Ok(true),
        // exit 1: pathspec matched nothing known to git.
        Err(GitError::Git { code: 1, .. }) => Ok(false),
        Err(other) => Err(other),
    }
}

/// Whether git ignores a single path (via `check-ignore -q --stdin`), as a
/// tri-state: `Ok(true)` = ignored (exit 0), `Ok(false)` = not ignored
/// (exit 1), `Err` = git failure (spawn failure or exit ≥ 128). The path
/// is fed as a pathname over stdin rather than an argv pathspec:
/// `check-ignore` supports no pathspec magic (any `:` prefix errors out),
/// and bare args are glob-expanded, so a file literally named `a[1].txt`
/// would otherwise be matched as a pattern instead of itself.
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
    match run_with_stdin(root, policy, &["check-ignore", "-q", "--stdin"], &relative) {
        Ok(GitOutput { code: Some(0), .. }) => Ok(true),
        Ok(GitOutput { code: Some(1), .. }) => Ok(false),
        Ok(_) => Ok(false),
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

/// Paths among `paths` that git knows (batch [`tracked`]).
///
/// One `git ls-files` spawn for the whole batch, each path wrapped in
/// `:(literal)` pathspec magic (see [`literal_pathspec`]) so glob-like
/// filenames match themselves. Returns the tracked subset in git's
/// output order, unquoted like [`changed_files`]'s enumeration; the
/// per-path [`tracked`] predicate is equivalent element-wise. Use this
/// over N [`tracked`] spawns whenever more than one path is in play —
/// batching is the point.
pub fn tracked_paths<P: AsRef<Path>>(root: &Path, paths: &[P]) -> Result<Vec<String>, GitError> {
    tracked_paths_with_policy(root, paths, EnvPolicy::Inherit)
}

/// [`tracked_paths`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn tracked_paths_with_policy<P: AsRef<Path>>(
    root: &Path,
    paths: &[P],
    policy: EnvPolicy,
) -> Result<Vec<String>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let specs = literal_specs(root, paths);
    let mut args = vec!["ls-files", "-z", "--"];
    args.extend(specs.iter().map(String::as_str));
    let out = run(root, policy, &args)?;
    Ok(String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect())
}

/// Build literal pathspecs (relative to `root`) for the batched query.
fn literal_specs<P: AsRef<Path>>(root: &Path, paths: &[P]) -> Vec<String> {
    paths
        .iter()
        .map(|path| literal_pathspec(&relative_to(root, path.as_ref())))
        .collect()
}

/// Paths among `paths` that git ignores (batched `check-ignore`).
///
/// One `git check-ignore --stdin -z` spawn: paths are fed NUL-separated
/// on stdin and the ignored subset is parsed from the NUL-separated
/// stdout (with `-z` git reports paths verbatim, no quoting). Exit 1
/// means "nothing is ignored" and yields an empty list; other non-zero
/// exits are typed errors. Git answers as it reads, so a large batch
/// can fill the child's stdout pipe while stdin is still being written;
/// the runner drains stdin on a writer thread to avoid that deadlock.
pub fn ignored_paths<P: AsRef<Path>>(root: &Path, paths: &[P]) -> Result<Vec<String>, GitError> {
    ignored_paths_with_policy(root, paths, EnvPolicy::Inherit)
}

/// [`ignored_paths`] with an explicit environment policy (policy-aware
/// variant; see [`run`] for what the policy controls).
pub fn ignored_paths_with_policy<P: AsRef<Path>>(
    root: &Path,
    paths: &[P],
    policy: EnvPolicy,
) -> Result<Vec<String>, GitError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let out = run_with_stdin(
        root,
        policy,
        &["check-ignore", "--stdin", "-z"],
        &z_path_input(root, paths),
    )?;
    Ok(z_paths(&out.stdout))
}

/// Parse NUL-separated verbatim paths (`-z` output, never quoted).
fn z_paths(stdout: &[u8]) -> Vec<String> {
    stdout
        .split(|b| *b == 0)
        .filter(|chunk| !chunk.is_empty())
        .map(|chunk| String::from_utf8_lossy(chunk).into_owned())
        .collect()
}

/// NUL-separated relative pathspecs for the batched `check-ignore` query.
fn z_path_input<P: AsRef<Path>>(root: &Path, paths: &[P]) -> String {
    let mut input = String::new();
    for path in paths {
        input.push_str(&relative_to(root, path.as_ref()));
        input.push('\0');
    }
    input
}

mod remote;
mod worktree;

pub use remote::{remote_key, remote_url, remote_url_with_policy};
pub use worktree::{
    WorktreeFacts, current_branch, current_branch_with_policy, worktree_facts,
    worktree_facts_with_policy,
};
