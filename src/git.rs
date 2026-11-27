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

/// Errors returned by the read-only git queries.
mod changes;
mod error;
mod query;
mod runner;

pub use changes::{
    changed_files, changed_files_between, changed_files_between_with_policy, changed_files_lossy,
    changed_files_with_policy, parse_porcelain, staged_files, staged_files_with_policy,
    uncommitted_files, uncommitted_files_lossy, uncommitted_files_with_policy,
};
pub use error::GitError;
pub use query::{
    PathStatus, WorktreeFacts, committed_content_hash, committed_content_hash_with_policy,
    content_hash, content_hash_with_policy, current_branch, current_branch_with_policy,
    ignored_paths, ignored_paths_with_policy, is_ignored, is_ignored_with_policy, path_status,
    path_status_with_policy, remote_key, remote_url, remote_url_with_policy, tracked,
    tracked_paths, tracked_paths_with_policy, tracked_with_policy, worktree_facts,
    worktree_facts_with_policy,
};
pub use runner::{EnvPolicy, GitOutput, repo_root, repo_root_from, run};
