use std::path::{Path, PathBuf};

use super::GitHooksError;

/// Locate the enclosing git repository by walking parent directories
/// until a `.git` entry is found.
///
/// Walks up from the current directory. Donor: pretender `repo_root()`.
pub fn repo_root() -> Result<PathBuf, GitHooksError> {
    let start = std::env::current_dir().map_err(|source| GitHooksError::Io {
        path: PathBuf::from("."),
        message: "failed to get current directory".to_string(),
        source,
    })?;
    repo_root_from(&start)
}

/// Like [`repo_root()`], but walks up from an explicit starting directory.
///
/// Exposed for tests and callers that already hold a working directory.
///
/// Alias delegating to [`crate::git::repo_root_from`] — the walk logic
/// moved into the shared `git` module (genesis-tpf); behavior is
/// unchanged, and `git::GitError::NotInRepo`/`Io` map back onto the
/// matching [`GitHooksError`] variants.
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitHooksError> {
    crate::git::repo_root_from(start).map_err(map_root_error)
}

/// Map a [`crate::git::GitError`] from the root walk onto the
/// [`GitHooksError`] variants callers of this module expect.
fn map_root_error(error: crate::git::GitError) -> GitHooksError {
    match error {
        crate::git::GitError::NotInRepo { start } => GitHooksError::NotInRepo { start },
        crate::git::GitError::Io {
            path,
            message,
            source,
        } => GitHooksError::Io {
            path,
            message,
            source,
        },
        // The root walk never spawns git, so `Git`/`Spawn` are unreachable;
        // still map them exhaustively onto `Io` for future-proofing.
        other => GitHooksError::Io {
            path: PathBuf::from("."),
            message: other.to_string(),
            source: std::io::Error::other(other.to_string()),
        },
    }
}

