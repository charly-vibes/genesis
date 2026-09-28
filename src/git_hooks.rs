//! Git hook primitives shared across the suite.
//!
//! Consolidates overlapping git-hook mechanics from three donors:
//!
//! - pretender (`src/main.rs`): install/uninstall with ownership marker,
//!   refuse-foreign-hook guard, `repo_root()` parent walk.
//! - wai (`src/commands/way/hooks.rs`): `core.hooksPath` resolution,
//!   hook-read helpers.
//! - espectacular (`src/init.rs`): lefthook framework detection and
//!   managed-block injection (rebuilt on [`crate::managed_block`]).
//!
//! Boundary note: this module contains no consuming-tool gate commands
//! (e.g. `ah check`, `pretender check`) — those stay in the tools.

use std::path::{Path, PathBuf};

use thiserror::Error;

/// Errors that can occur while resolving git repository roots or hook
/// directories.
#[derive(Debug, Error)]
pub enum GitHooksError {
    /// No `.git` entry was found in the starting directory or any parent.
    #[error("not inside a git repository: no .git entry found in {start} or any parent", start = start.display())]
    NotInRepo {
        /// The directory the upward walk started from.
        start: PathBuf,
    },
    /// An I/O or process error during a git or filesystem operation.
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
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitHooksError> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err(GitHooksError::NotInRepo {
        start: start.to_path_buf(),
    })
}

/// Resolve the repository's hook directory.
///
/// Returns the local `core.hooksPath` when set (relative paths are
/// resolved against `root`, matching git's semantics), else the default
/// `.git/hooks`. Donor: wai `git_core_hooks_path()`.
pub fn resolve_hooks_dir(root: &Path) -> Result<PathBuf, GitHooksError> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "config",
            "--local",
            "core.hooksPath",
        ])
        .output()
        .map_err(|source| GitHooksError::Io {
            path: root.to_path_buf(),
            message: "failed to run `git config --local core.hooksPath`".to_string(),
            source,
        })?;
    // Non-zero exit means the config is unset — fall back to the default.
    let hooks_path = if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty() { None } else { Some(val) }
    } else {
        None
    };
    match hooks_path {
        // Relative paths resolve against the repo root (git's own
        // semantics for core.hooksPath).
        Some(p) if Path::new(&p).is_relative() => Ok(root.join(p)),
        Some(p) => Ok(PathBuf::from(p)),
        None => Ok(root.join(".git/hooks")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    // -- Repository root discovery ---------------------------------------

    #[test]
    fn repo_root_finds_root_from_nested_dir() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let nested = fixture.path("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        let found = repo_root_from(&nested).unwrap();
        assert_eq!(found, fixture.root());
    }

    #[test]
    fn repo_root_from_the_root_itself_works() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let found = repo_root_from(fixture.root()).unwrap();
        assert_eq!(found, fixture.root());
    }

    #[test]
    fn repo_root_errors_outside_a_repository() {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let err = repo_root_from(fixture.root()).unwrap_err();
        assert!(matches!(err, GitHooksError::NotInRepo { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("not inside a git repository"),
            "error should name the problem: {msg}"
        );
    }

    // -- Hooks directory resolution --------------------------------------

    #[test]
    fn resolve_hooks_dir_defaults_to_git_hooks() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let resolved = resolve_hooks_dir(fixture.root()).unwrap();
        assert_eq!(resolved, fixture.root().join(".git/hooks"));
    }

    #[test]
    fn resolve_hooks_dir_honors_local_core_hooks_path() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let out = fixture
            .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
            .unwrap();
        assert!(out.success(), "git config failed: {}", out.stderr);
        let resolved = resolve_hooks_dir(fixture.root()).unwrap();
        assert_eq!(resolved, fixture.root().join(".githooks"));
    }
}
