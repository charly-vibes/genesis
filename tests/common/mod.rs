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
//! Deliberate child-process poisoning: some tests spawn themselves as
//! children with hook-context variables *deliberately* set (the
//! `GENESIS_*_CHILD` sentinel marks them) to assert inherit-vs-strip
//! behavior. The strip is skipped in those children so the poison
//! survives.

#[cfg(unix)]
#[used]
#[unsafe(link_section = ".init_array")]
static STRIP_HOOK_GIT_ENV: extern "C" fn() = strip_hook_git_env;

#[cfg(unix)]
extern "C" fn strip_hook_git_env() {
    // Child processes of deliberate-poison tests keep their environment:
    // their parent marked them with a sentinel.
    for sentinel in [
        "GENESIS_GIT_TEST_CHILD",
        "GENESIS_GIT_POLICY_CHILD",
        "GENESIS_FIXTURE_ENV_CHILD",
        "GENESIS_GIT_HOOKS_ENV_CHILD",
    ] {
        if std::env::var_os(sentinel).is_some_and(|v| !v.is_empty()) {
            return;
        }
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
