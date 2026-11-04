//! Command execution results from [`Fixture::run`], with hermetic
//! environment plumbing.

use std::process::Command;

/// The result of running a command in a fixture.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
    /// Exit code (or -1 if the process was terminated by a signal).
    pub exit_code: i32,
    /// On Unix, the terminating signal if the process was killed by one;
    /// `None` otherwise (including on non-Unix platforms).
    pub signal: Option<i32>,
}

/// Remove the hook-context variables git exports to hook processes
/// (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE`) from a fixture command.
///
/// Rationale: when the test suite runs inside a git hook (the pre-commit
/// testaruda selection runs the whole suite), the surrounding `git
/// commit` exports `GIT_DIR` — and a fixture that inherits it would run
/// its `git init`/`add`/`commit` against the *real* repository instead
/// of the fixture's own (observed: a hook-time cargo test landed a bogus
/// `initial` commit on a development branch). Fixtures are self-contained
/// sandboxes; their git operations must discover the fixture repo from
/// the command's working directory alone.
pub(super) fn hermetic_git(cmd: &mut Command) -> &mut Command {
    for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
        cmd.env_remove(var);
    }
    cmd
}

/// Extract the terminating signal from an [`ExitStatus`], if any.
#[cfg(unix)]
pub(super) fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}
