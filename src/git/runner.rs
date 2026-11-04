//! The canonical git runner with opt-in hook-context env hygiene, and
//! walk-based repository-root detection that ignores `GIT_DIR` state.

use std::path::{Path, PathBuf};

use super::error::GitError;

/// The outcome of a successful git subprocess spawn.
#[derive(Debug, Clone)]
pub struct GitOutput {
    /// Exit code of the git process (`Some(0)` for a successful run;
    /// `None` when the process was terminated by a signal).
    pub code: Option<i32>,
    /// Captured stdout bytes.
    pub stdout: Vec<u8>,
    /// Captured stderr bytes.
    pub stderr: Vec<u8>,
}

/// Environment policy for the canonical git runner.
///
/// Hook harnesses legitimately inject `GIT_DIR`, `GIT_INDEX_FILE`, and
/// `GIT_WORK_TREE` into hook processes, so inheritance is the default;
/// stripping is opt-in for callers that need a clean view of the
/// repository.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EnvPolicy {
    /// Pass the caller's environment through unchanged (default).
    #[default]
    Inherit,
    /// Remove `GIT_DIR`, `GIT_INDEX_FILE`, and `GIT_WORK_TREE` before
    /// spawning, so git discovers the repository from `root` alone.
    StripHookContext,
}

impl EnvPolicy {
    /// The hook-injected variables [`EnvPolicy::StripHookContext`] removes.
    const HOOK_CONTEXT_VARS: [&str; 3] = ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"];
}

/// Run the git binary in `root` with the given environment policy.
///
/// This is the module's canonical runner: `root` becomes the child
/// process's working directory (pass paths relative to `root`), and the
/// policy decides whether hook-context variables stay in the child's
/// environment. Non-zero exits yield [`GitError::Git`]; spawn failures
/// (e.g. git missing from `PATH`) yield [`GitError::Spawn`].
pub fn run(root: &Path, policy: EnvPolicy, args: &[&str]) -> Result<GitOutput, GitError> {
    let output = spawn_git(root, policy, args)?;
    let code = output.status.code();
    if code != Some(0) {
        return Err(GitError::Git {
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            code: code.unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }
    Ok(GitOutput {
        code,
        stdout: output.stdout,
        stderr: output.stderr,
    })
}

/// Spawn git in `root` with the policy's env hygiene applied.
fn spawn_git(
    root: &Path,
    policy: EnvPolicy,
    args: &[&str],
) -> Result<std::process::Output, GitError> {
    let mut cmd = std::process::Command::new("git");
    cmd.args(args).current_dir(root);
    if policy == EnvPolicy::StripHookContext {
        for var in EnvPolicy::HOOK_CONTEXT_VARS {
            cmd.env_remove(var);
        }
    }
    cmd.output().map_err(|source| GitError::Spawn { source })
}

/// Split command stdout into non-empty trimmed lines.
pub(super) fn lines_of(stdout: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
        .collect()
}

/// Whether a 128-exit stderr indicates HEAD does not resolve (unborn HEAD
/// in a repository without commits).
pub(super) fn is_unborn_head(stderr: &str) -> bool {
    stderr.contains("unknown revision") || stderr.contains("bad revision")
}

/// Locate the enclosing git repository by walking parent directories from
/// the current directory until a `.git` entry (file or directory) is found.
///
/// The walk never invokes git and therefore is not influenced by `GIT_DIR`
/// or any other environment state.
pub fn repo_root() -> Result<PathBuf, GitError> {
    let start = std::env::current_dir().map_err(|source| GitError::Io {
        path: PathBuf::from("."),
        message: "failed to get current directory".to_string(),
        source,
    })?;
    repo_root_from(&start)
}

/// Locate the enclosing git repository by walking parent directories from
/// an explicit starting directory until a `.git` entry (file or directory)
/// is found.
///
/// A `.git` file (linked-worktree marker) counts as a repository root, and
/// starting the walk inside a `.git` directory resolves to the enclosing
/// repository root.
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitError> {
    let root = start
        .ancestors()
        .find(|dir| dir.join(".git").exists())
        .ok_or_else(|| GitError::NotInRepo {
            start: start.to_path_buf(),
        })?;
    Ok(root.to_path_buf())
}
