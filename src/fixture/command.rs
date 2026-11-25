//! Command execution results from [`Fixture::run`], with hermetic
//! environment plumbing.

use serde::de::DeserializeOwned;
use std::path::Path;
use std::process::Command;

use super::FixtureError;

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

impl CommandOutput {
    /// Build a [`CommandOutput`] from a finished process.
    pub(super) fn from_process(output: &std::process::Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            signal: signal_of(&output.status),
        }
    }

    /// Parse stdout as JSON.
    pub fn json<T: DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&self.stdout).ok()
    }

    /// Returns `true` if the exit code is 0.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Spawn `program args` in `dir`, hermetically (see [`hermetic_git`]); spawn
/// failure maps to [`FixtureError::Spawn`].
pub(super) fn spawn_in(
    program: &str,
    args: &[&str],
    dir: &Path,
) -> Result<std::process::Output, FixtureError> {
    hermetic_git(&mut Command::new(program))
        .args(args)
        .current_dir(dir)
        .output()
        .map_err(|e| FixtureError::Spawn {
            program: program.to_string(),
            source: e,
        })
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

/// Extract the terminating signal from an exit status, if any.
#[cfg(unix)]
fn signal_of(status: &std::process::ExitStatus) -> Option<i32> {
    use std::os::unix::process::ExitStatusExt;
    status.signal()
}

/// Non-Unix exit statuses carry no signal information.
#[cfg(not(unix))]
fn signal_of(_status: &std::process::ExitStatus) -> Option<i32> {
    None
}
