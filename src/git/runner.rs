//! The canonical git runner with opt-in hook-context env hygiene, and
//! walk-based repository-root detection that ignores `GIT_DIR` state.

use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::thread::JoinHandle;

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
    git_output(args, output, &[0])
}

/// Run the git binary in `root` with the policy's env hygiene applied,
/// feeding `input` on stdin and returning the raw outcome.
///
/// Unlike [`run`], both 0 and 1 exits are returned to the caller
/// (`Ok`) because some git commands use them as meaningful answers
/// (`check-ignore` maps 1 to "a given path is ignored"); only other
/// non-zero exits yield [`GitError::Git`].
///
/// Stdin is drained on a thread: git answers incrementally, so a large
/// batch can fill the child's stdout pipe while stdin is still being
/// written — writing inline would deadlock there.
///
/// A broken pipe on the stdin write means git exited before consuming
/// the input; the child's own exit status is the authoritative failure
/// and maps through [`git_output`] to [`GitError::Git`]. Other stdin
/// write failures still yield [`GitError::Io`].
pub(super) fn run_with_stdin(
    root: &Path,
    policy: EnvPolicy,
    args: &[&str],
    input: &str,
) -> Result<GitOutput, GitError> {
    let mut child = spawn_stdin_git(root, policy, args)?;
    let writer = spawn_stdin_writer(child.stdin.take().expect("stdin piped"), input);
    let output = child
        .wait_with_output()
        .map_err(|source| GitError::Spawn { source })?;
    let written = writer.join().expect("stdin writer thread panicked");
    if let Some(err) = written
        .err()
        .and_then(|source| stdin_write_error(root, source))
    {
        return Err(err);
    }
    git_output(args, output, &[0, 1])
}

/// Classify a failed stdin write: a broken pipe means git exited before
/// consuming the input (the child's exit status is authoritative —
/// fall through to [`git_output`]); anything else is an [`GitError::Io`].
fn stdin_write_error(root: &Path, source: std::io::Error) -> Option<GitError> {
    if source.kind() == std::io::ErrorKind::BrokenPipe {
        None
    } else {
        Some(GitError::Io {
            path: root.to_path_buf(),
            message: "failed to write git stdin".to_string(),
            source,
        })
    }
}

/// Spawn git with piped stdio for [`run_with_stdin`].
fn spawn_stdin_git(root: &Path, policy: EnvPolicy, args: &[&str]) -> Result<Child, GitError> {
    stdin_git_command(root, policy, args)
        .spawn()
        .map_err(|source| GitError::Spawn { source })
}

/// The piped-stdio git command: all three streams piped so stdout drains
/// while stdin is still being written (the large-batch deadlock case).
fn stdin_git_command(root: &Path, policy: EnvPolicy, args: &[&str]) -> Command {
    let mut cmd = git_command(root, policy, args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());
    cmd
}

/// Drain `input` into the child's stdin on a thread.
fn spawn_stdin_writer(stdin: ChildStdin, input: &str) -> JoinHandle<std::io::Result<()>> {
    use std::io::Write as _;
    let input = input.to_string();
    std::thread::spawn(move || -> std::io::Result<()> {
        let mut stdin = stdin;
        stdin.write_all(input.as_bytes())?;
        stdin.flush()
    })
}

/// Spawn git in `root` with the policy's env hygiene applied.
fn spawn_git(
    root: &Path,
    policy: EnvPolicy,
    args: &[&str],
) -> Result<std::process::Output, GitError> {
    git_command(root, policy, args)
        .output()
        .map_err(|source| GitError::Spawn { source })
}

/// A `git` command in `root` with the policy's env hygiene applied.
fn git_command(root: &Path, policy: EnvPolicy, args: &[&str]) -> Command {
    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(root);
    if policy == EnvPolicy::StripHookContext {
        for var in EnvPolicy::HOOK_CONTEXT_VARS {
            cmd.env_remove(var);
        }
    }
    cmd
}

/// Map a finished git output onto `GitOutput`, failing on exits outside
/// `allowed` (`run` accepts only 0; `run_with_stdin` also accepts 1 —
/// some git commands use it as a meaningful answer, e.g. `check-ignore`
/// maps 1 to "a given path is ignored").
fn git_output(
    args: &[&str],
    output: std::process::Output,
    allowed: &[i32],
) -> Result<GitOutput, GitError> {
    let code = output.status.code();
    if !allowed.contains(&code.unwrap_or(-1)) {
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
