use std::path::PathBuf;

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
    /// An existing hook file does not carry the ownership marker — it is
    /// not ours to modify.
    #[error("refusing to {action} hook not installed with this marker: {path}", path = path.display())]
    ForeignHook {
        /// The hook file that was refused.
        path: PathBuf,
        /// Operation being performed: `overwrite` or `remove`.
        action: &'static str,
    },
    /// `git config` reports the queried directory is not a git repository
    /// (exit 128). Scope queries must surface this instead of treating it
    /// as "unset" and walking on to global/system config (genesis-5sm).
    #[error("not a git repository: {}", path.display())]
    NotARepository {
        /// The directory the `git config` query was anchored at.
        path: PathBuf,
    },
    /// `git config` could not parse the repository config (exit 128, e.g.
    /// a corrupt `.git/config`) — the effective `core.hooksPath` is
    /// unknowable, so scope queries refuse to guess (genesis-5sm).
    #[error("git config failed at {}: {}", path.display(), message)]
    ConfigParseFailed {
        /// The directory the `git config` query was anchored at.
        path: PathBuf,
        /// git's stderr explaining the failure.
        message: String,
    },
    /// A `HookName::Other` name carried path components (`..`, a root) —
    /// refused rather than joined into the hooks directory (genesis-5sm).
    #[error("invalid hook name {name:?}: expected a plain hook file name, not a path")]
    InvalidHookName {
        /// The refused name.
        name: String,
    },
    /// A managed-block marker is present but its pair is not — the
    /// config is in an unknown state and wiring refuses to guess; the
    /// file is left unmodified.
    #[error(
        "unbalanced managed-block markers in {}: found one of the '{start}'/'{end}' pair but not both — fix or remove the markers manually",
        path.display()
    )]
    UnbalancedLefthookMarkers {
        /// Config file with the unbalanced markers.
        path: PathBuf,
        /// The start marker that was searched.
        start: String,
        /// The end marker that was searched.
        end: String,
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
    /// No `lefthook.yml` or `lefthook.yaml` exists — wiring refuses to create
    /// a config from scratch (design D4 non-goal).
    #[error(
        "no supported hook framework config found in {root}: expected lefthook.yml or lefthook.yaml"
    )]
    MissingLefthookConfig {
        /// Repository root that was searched.
        root: PathBuf,
    },
    /// The lefthook config cannot be anchored (e.g. the stage key is quoted
    /// or the structure is unrecognized) — the file is left unmodified.
    #[error(
        "cannot anchor stage '{stage}' in {}: {message} — file left unmodified",
        path.display()
    )]
    UnanchorableLefthookConfig {
        /// The config file that was not modified.
        path: PathBuf,
        /// Stage key that could not be anchored.
        stage: String,
        /// What went wrong while looking for the anchor.
        message: String,
    },
}
