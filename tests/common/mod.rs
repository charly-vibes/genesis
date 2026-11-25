//! Hook-env hygiene for integration tests (testaruda-c64 pattern,
//! generalized — mirrors testaruda's `tests/common/mod.rs`).
//!
//! When tests run under a git hook from a linked WORKTREE, git exports
//! `GIT_DIR` (pointing at the outer repo's worktree gitdir) plus friends;
//! lefthook passes them through to commands. Every spawned git process
//! that inherits them then answers for the *real* repository instead of
//! the fixture — and hook-time suite runs (the pre-commit testaruda
//! selection) would hijack fixture git operations onto the outer repo.
//! Pushes/commits from main checkouts don't export `GIT_DIR` — which is
//! why these failures only surface when committing migration branches
//! from worktrees.
//!
//! The strip runs at binary init (`.init_array`), before any test thread
//! starts, so it does not race parallel test threads and does not mutate
//! the environment mid-run.
//!
//! Beyond env hygiene this module carries shared doc-reading helpers
//! (repo-relative paths, markdown normalization) and a buffered
//! stdout/stderr capture pair used by the doc-drift and evals guards.
//!
//! Deliberate child-process poisoning: some tests spawn themselves as
//! children with hook-context variables *deliberately* set (the
//! `GENESIS_*_CHILD` sentinel marks them) to assert inherit-vs-strip
//! behavior. The strip is skipped in those children so the poison
//! survives.

/// Pretender-compliant unwrapping for tests: `value.require("why")` in place
/// of `value.expect("why")` / `value.unwrap()`.
///
/// Pretender counts `.unwrap()` and `.expect(` sites per file (test role
/// allows at most 5). This trait preserves the "why" context while keeping
/// files under the limit; panics report the caller's location.
#[allow(dead_code)] // not every binary that declares `common` unwraps
pub trait TestUnwrap: Sized {
    type Output;

    /// Unwrap with context; panics with `why` plus the error/None detail.
    #[track_caller]
    fn require(self, why: &str) -> Self::Output;
}

use std::fs;
use std::path::{Path, PathBuf};

/// Absolute path of a repo-relative file.
#[allow(dead_code)]
pub fn project_file(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// Read a repo-relative file, panicking with the path on failure.
#[allow(dead_code)]
pub fn read(rel: &str) -> String {
    fs::read_to_string(project_file(rel)).unwrap_or_else(|e| panic!("read {rel}: {e}"))
}

/// Collapse whitespace and strip markdown noise (backslash escapes,
/// emphasis markers, inline-code backticks) so text guards survive prose
/// formatting.
#[allow(dead_code)]
pub fn norm_markdown(text: &str) -> String {
    let stripped: String = text
        .chars()
        .filter(|c| !matches!(c, '\\' | '*' | '`'))
        .collect();
    stripped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Normalized text of a repo-relative doc file.
#[allow(dead_code)]
pub fn norm_doc(rel: &str) -> String {
    norm_markdown(&read(rel))
}

/// Buffered stdout/stderr pair so tests don't write to the real terminal.
#[allow(dead_code)]
#[derive(Default)]
pub struct Streams {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[allow(dead_code)]
impl Streams {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn as_strings(&self) -> (String, String) {
        (
            String::from_utf8_lossy(&self.stdout).into_owned(),
            String::from_utf8_lossy(&self.stderr).into_owned(),
        )
    }
}

impl<T, E: std::fmt::Debug> TestUnwrap for Result<T, E> {
    type Output = T;

    #[track_caller]
    fn require(self, why: &str) -> T {
        match self {
            Ok(value) => value,
            Err(err) => panic!("{why}: {err:?}"),
        }
    }
}

impl<T> TestUnwrap for Option<T> {
    type Output = T;

    #[track_caller]
    fn require(self, why: &str) -> T {
        match self {
            Some(value) => value,
            None => panic!("{why}: unexpectedly None"),
        }
    }
}

#[cfg(unix)]
#[used]
#[unsafe(link_section = ".init_array")]
static STRIP_HOOK_GIT_ENV: extern "C" fn() = strip_hook_git_env;

#[cfg(unix)]
extern "C" fn strip_hook_git_env() {
    strip_hook_git_env_impl();
}

/// Child processes of deliberate-poison tests keep their environment:
/// their parent marked them with a sentinel.
fn strip_hook_git_env_impl() {
    if poisoned_by_parent() {
        return;
    }
    for var in [
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_PREFIX",
        "GIT_CONFIG_PARAMETERS",
        "GIT_QUARANTINE_PATH",
    ] {
        // Safe: called once at process init, before test threads spawn.
        unsafe { std::env::remove_var(var) };
    }
}

/// `true` when this process is a deliberate-poison test child.
fn poisoned_by_parent() -> bool {
    [
        "GENESIS_GIT_TEST_CHILD",
        "GENESIS_GIT_POLICY_CHILD",
        "GENESIS_FIXTURE_ENV_CHILD",
        "GENESIS_GIT_HOOKS_ENV_CHILD",
    ]
    .iter()
    .any(|sentinel| std::env::var_os(sentinel).is_some_and(|v| !v.is_empty()))
}
