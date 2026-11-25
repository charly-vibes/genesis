//! Errors surfaced by fixture construction and in-fixture command execution.

use std::path::PathBuf;

/// Errors that can occur while building a fixture or running a command in one.
#[derive(Debug, thiserror::Error)]
pub enum FixtureError {
    /// An empty command was passed to [`Fixture::run`].
    #[error("cannot run an empty command (no program given)")]
    EmptyCommand,
    /// A subprocess failed to spawn.
    #[error("failed to spawn `{program}`")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    /// `git` is not installed or a git operation failed.
    #[error("git operation `{stage}` failed")]
    Git {
        stage: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// An I/O error while laying out the fixture on disk.
    #[error("io error at {}: {message}", path.display())]
    Io { path: PathBuf, message: String },
    /// A config could not be serialized or written.
    #[error("config error")]
    Config(#[from] crate::config::ConfigError),
}
