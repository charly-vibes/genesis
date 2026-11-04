//! Errors returned by the read-only git queries.

use std::path::PathBuf;

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
