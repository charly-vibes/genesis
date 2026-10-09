//! Read-only git plumbing shared across the suite.
//!
//! Purpose: one canonical, subprocess-only substrate for the git queries
//! every tool in the charly-vibes suite previously re-implemented
//! (repo-root walks, changed-file enumeration, tracked/ignored checks,
//! content hashes).
//!
//! Responsibilities:
//!
//! - walk-based repository-root detection that ignores `GIT_DIR` state
//! - one canonical git runner ([`run`]) with opt-in hook-context env
//!   hygiene ([`EnvPolicy`])
//! - read-only queries with declared failure semantics: typed errors
//!   everywhere, silent degradation only in explicitly named `*_lossy`
//!   variants
//!
//! Rationale / boundary: this module owns read-only git mechanics only —
//! no `git init`/`add`/`commit` (write paths stay with callers), no hook
//! wiring (see [`crate::git_hooks`]), and no git library linkage (the git
//! binary is spawned as a subprocess). Donors: pretender `repo_root()`
//! walk, dont `main.rs` dirty/content-hash checks, testaruda `change.rs`
//! porcelain parsing, espectacular `changed_files_from_git`.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors returned by the read-only git queries.
#[derive(Debug, Error)]
pub enum GitError {
    /// No `.git` entry was found in the starting directory or any parent.
    #[error(
        "not inside a git repository: no .git entry found in {start} or any parent",
        start = start.display()
    )]
    NotInRepo {
        /// The directory the upward walk started from.
        start: PathBuf,
    },
    /// A git subprocess exited non-zero.
    #[error("git {} failed with exit code {code}: {stderr}", args.join(" "))]
    Git {
        /// The arguments the failing invocation was built from.
        args: Vec<String>,
        /// Non-zero exit code of the git process.
        code: i32,
        /// Captured stderr (trimmed).
        stderr: String,
    },
    /// The git binary could not be spawned at all.
    #[error("failed to spawn git: {source}")]
    Spawn {
        /// Underlying spawn error.
        #[source]
        source: std::io::Error,
    },
    /// A filesystem error (e.g. resolving the current directory).
    #[error("io error at {}: {message}", path.display())]
    Io {
        /// Path the operation touched.
        path: PathBuf,
        /// Human-readable context.
        message: String,
        /// Underlying error.
        #[source]
        source: std::io::Error,
    },
}

/// Locate the enclosing git repository by walking parent directories from
/// the current directory until a `.git` entry (file or directory) is found.
///
/// The walk never invokes git and therefore is not influenced by `GIT_DIR`
/// or any other environment state.
pub fn repo_root() -> Result<PathBuf, GitError> {
    let start = std::env::current_dir().map_err(|source| GitError::Io {
        path: PathBuf::from("."),
        message: "failed to get current directory".to_string(),
        source,
    })?;
    repo_root_from(&start)
}

/// Locate the enclosing git repository by walking parent directories from
/// an explicit starting directory until a `.git` entry (file or directory)
/// is found.
///
/// A `.git` file (linked-worktree marker) counts as a repository root, and
/// starting the walk inside a `.git` directory resolves to the enclosing
/// repository root.
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitError> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err(GitError::NotInRepo {
        start: start.to_path_buf(),
    })
}

/// The outcome of a successful git subprocess spawn.
#[derive(Debug, Clone)]
pub struct GitOutput {
    /// Exit code of the git process (`Some(0)` for a successful run;
    /// `None` when the process was terminated by a signal).
    pub code: Option<i32>,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
}

/// Environment policy for the canonical git runner.
///
/// Hook harnesses legitimately inject `GIT_DIR`, `GIT_INDEX_FILE`, and
/// `GIT_WORK_TREE` into hook processes, so inheritance is the default;
/// stripping is opt-in for callers that need a clean view of the
/// repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvPolicy {
    /// Pass the caller's environment through unchanged (default).
    #[default]
    Inherit,
    /// Remove `GIT_DIR`, `GIT_INDEX_FILE`, and `GIT_WORK_TREE` before
    /// spawning, so git discovers the repository from `root` alone.
    StripHookContext,
}

impl EnvPolicy {
    /// The hook-injected variables [`EnvPolicy::StripHookContext`] removes.
    const HOOK_CONTEXT_VARS: [&str; 3] = ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"];
}

/// Run the git binary in `root` with the given environment policy.
///
/// This is the module's canonical runner: `root` becomes the child
/// process's working directory (pass paths relative to `root`), and the
/// policy decides whether hook-context variables stay in the child's
/// environment. Non-zero exits yield [`GitError::Git`]; spawn failures
/// (e.g. git missing from `PATH`) yield [`GitError::Spawn`].
pub fn run(root: &Path, policy: EnvPolicy, args: &[&str]) -> Result<GitOutput, GitError> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args).current_dir(root);
    if policy == EnvPolicy::StripHookContext {
        for var in EnvPolicy::HOOK_CONTEXT_VARS {
            cmd.env_remove(var);
        }
    }
    let output = cmd.output().map_err(|source| GitError::Spawn { source })?;
    let code = output.status.code();
    if code != Some(0) {
        return Err(GitError::Git {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            code: code.unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(GitOutput {
        code,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Split command stdout into non-empty trimmed lines.
fn lines_of(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Whether a 128-exit stderr indicates HEAD does not resolve (unborn HEAD
/// in a repository without commits).
fn is_unborn_head(stderr: &str) -> bool {
    stderr.contains("unknown revision") || stderr.contains("bad revision")
}

/// Enumerate untracked files (git's own ignore rules respected).
fn untracked_files(root: &Path) -> Result<Vec<String>, GitError> {
    let out = run(
        root,
        EnvPolicy::Inherit,
        &["ls-files", "--others", "--exclude-standard"],
    )?;
    Ok(lines_of(&out.stdout))
}

/// Files with tracked changes against `HEAD` (staged or unstaged).
///
/// In a repository with no commits (unborn HEAD, where `git diff HEAD`
/// exits 128) this falls back to enumerating untracked files rather than
/// returning an empty set — the unborn-HEAD fallback is the only mode in
/// which untracked files appear here.
pub fn changed_files(root: &Path) -> Result<Vec<String>, GitError> {
    match run(root, EnvPolicy::Inherit, &["diff", "--name-only", "HEAD"]) {
        Ok(out) => Ok(lines_of(&out.stdout)),
        Err(GitError::Git {
            code: 128, stderr, ..
        }) if is_unborn_head(&stderr) => untracked_files(root),
        Err(other) => Err(other),
    }
}

/// Files with tracked changes between two revisions (both must resolve).
pub fn changed_files_between(root: &Path, base: &str, head: &str) -> Result<Vec<String>, GitError> {
    let out = run(
        root,
        EnvPolicy::Inherit,
        &["diff", "--name-only", base, head],
    )?;
    Ok(lines_of(&out.stdout))
}

/// Every entry of `git status --porcelain` (v1): staged, unstaged,
/// renamed (new path), and untracked files.
pub fn uncommitted_files(root: &Path) -> Result<Vec<String>, GitError> {
    let out = run(root, EnvPolicy::Inherit, &["status", "--porcelain"])?;
    Ok(parse_porcelain(&String::from_utf8_lossy(&out.stdout)))
}

/// Like [`uncommitted_files`], but degrades silently: on any git failure
/// (non-repo directory, spawn failure, non-zero exit) returns an empty
/// set instead of an error. Use only where an empty result is an
/// acceptable answer; the degradation is contractual and explicit.
pub fn uncommitted_files_lossy(root: &Path) -> Vec<String> {
    uncommitted_files(root).unwrap_or_default()
}

/// Parse a `git status --porcelain` (v1) body into file paths.
///
/// Plain `XY path` records pass through; rename/copy records
/// `XY old -> new` report only the new path (never the joined string);
/// quoted paths (`core.quotePath`) are unquoted, including C-style
/// escapes (`\"`, `\\`, `\t`, `\n`, octal `\NNN` byte values); an
/// empty body parses to an empty list.
pub fn parse_porcelain(body: &str) -> Vec<String> {
    let mut files = Vec::new();
    for line in body.lines() {
        let bytes = line.as_bytes();
        // A record needs at least two status chars, a space, and a path.
        if bytes.len() < 4 || bytes[2] != b' ' {
            continue;
        }
        let raw = &line[3..];
        // Rename/copy records carry `old -> new`; report the new path
        // whether the rename is staged (X) or in the worktree column (Y).
        let is_rename_or_copy =
            bytes[0] == b'R' || bytes[0] == b'C' || bytes[1] == b'R' || bytes[1] == b'C';
        let path_part = if is_rename_or_copy {
            match raw.rsplit_once(" -> ") {
                Some((_old, new)) => new,
                None => raw,
            }
        } else {
            raw
        };
        let path = if path_part.len() >= 2 && path_part.starts_with('"') && path_part.ends_with('"')
        {
            unquote_c_style(&path_part[1..path_part.len() - 1])
        } else {
            path_part.to_string()
        };
        if !path.is_empty() {
            files.push(path);
        }
    }
    files
}

/// Decode git's C-style path quoting (the content between the surrounding
/// double quotes) byte by byte. Octal escapes are raw byte values, so the
/// decoded bytes are lossily converted to UTF-8 at the end.
fn unquote_c_style(inner: &str) -> String {
    let bytes = inner.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 1;
            match bytes[i] {
                b'"' => out.push(b'"'),
                b'\'' => out.push(b'\''),
                b'\\' => out.push(b'\\'),
                b't' => out.push(b'\t'),
                b'n' => out.push(b'\n'),
                b'r' => out.push(b'\r'),
                b'a' => out.push(0x07),
                b'b' => out.push(0x08),
                b'f' => out.push(0x0C),
                b'v' => out.push(0x0B),
                digit @ b'0'..=b'7' => {
                    // Up to three octal digits encode one byte.
                    let mut value = u32::from(digit - b'0');
                    let mut taken = 1;
                    while taken < 3 && i + 1 < bytes.len() && (b'0'..=b'7').contains(&bytes[i + 1])
                    {
                        i += 1;
                        value = value * 8 + u32::from(bytes[i] - b'0');
                        taken += 1;
                    }
                    out.push(value as u8);
                }
                other => out.push(other),
            }
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}
/// Path queries run with `root` as the working directory, so git needs
/// path arguments relative to `root`. Callers may pass absolute paths
/// under `root`; those are relativized, anything else passes through.
fn relative_to(root: &Path, path: &Path) -> String {
    if let Ok(rest) = path.strip_prefix(root) {
        rest.to_string_lossy().into_owned()
    } else {
        path.to_string_lossy().into_owned()
    }
}

/// Whether the repository knows a path (via `git ls-files`), independent
/// of working-tree modifications. `Ok(false)` for paths git does not
/// know; a typed error for any other failure.
pub fn tracked(root: &Path, path: &Path) -> Result<bool, GitError> {
    let relative = relative_to(root, path);
    match run(
        root,
        EnvPolicy::Inherit,
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
    let relative = relative_to(root, path);
    match run(
        root,
        EnvPolicy::Inherit,
        &["check-ignore", "-q", "--", &relative],
    ) {
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
    let relative = relative_to(root, path);
    let out = run(root, EnvPolicy::Inherit, &["hash-object", "--", &relative])?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Resolve the committed content of a path via `git rev-parse
/// HEAD:<path>` — the blob hash of the file as committed at HEAD.
pub fn committed_content_hash(root: &Path, path: &Path) -> Result<String, GitError> {
    let spec = format!("HEAD:{}", relative_to(root, path));
    let out = run(root, EnvPolicy::Inherit, &["rev-parse", "--verify", &spec])?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}
