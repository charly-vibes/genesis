use std::path::{Path, PathBuf};

use super::GitHooksError;

/// Resolve the repository's hook directory.
///
/// Resolves `core.hooksPath` across all config scopes (local → global →
/// system), matching git's own precedence (genesis-c64); relative values
/// resolve against `root`. When no scope sets `core.hooksPath`, returns
/// the default `.git/hooks`. When `core.hooksPath` is set to the empty
/// string (git disables hooks), falls back to the default — consumers
/// that need to detect the disabled case should use
/// [`effective_hooks_dir`].
///
/// Donor: wai `git_core_hooks_path()`.
pub fn resolve_hooks_dir(root: &Path) -> Result<PathBuf, GitHooksError> {
    resolve_hooks_dir_with_env(root, &[])
}

/// Like [`resolve_hooks_dir`], but passes extra environment variables to
/// the spawned `git config` process. Test seam (see [`effective_hooks_dir_with_env`]).
pub(crate) fn resolve_hooks_dir_with_env(
    root: &Path,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    let effective = effective_hooks_dir_with_env(root, envs)?;
    if effective.scope == HooksDirScope::Disabled {
        return Ok(root.join(".git/hooks"));
    }
    Ok(effective.path)
}

/// Which config scope an effective `core.hooksPath` value was found in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HooksDirScope {
    /// Set in the repository's local config (`.git/config`).
    Local,
    /// Set in the user's global config (XDG or `$HOME/.gitconfig`).
    Global,
    /// Set in the system config.
    System,
    /// Unset everywhere — git's default `.git/hooks`.
    Default,
    /// `core.hooksPath` is set to the empty string — git disables hooks
    /// entirely. [`EffectiveHooksDir::path`] is empty in this case.
    Disabled,
}

/// The hooks directory git will actually use, plus the scope the setting
/// came from. Consumers (doctor-style checks) use [`HooksDirScope`] to
/// warn when a hook was installed at a different scope than the one git
/// will invoke hooks from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveHooksDir {
    /// Resolved hooks directory. Empty when the scope is [`HooksDirScope::Disabled`].
    pub path: PathBuf,
    /// Where the effective value was found.
    pub scope: HooksDirScope,
}

/// Resolve `core.hooksPath` across all config scopes (local → global →
/// system) and report which scope the effective value came from.
///
/// Relative values resolve against `root` for any scope (git runs hooks
/// with the repository root as the working directory in the common
/// case). An empty-string value is reported as
/// [`HooksDirScope::Disabled`] rather than falling back to the default.
pub fn effective_hooks_dir(root: &Path) -> Result<EffectiveHooksDir, GitHooksError> {
    effective_hooks_dir_with_env(root, &[])
}

/// Like [`effective_hooks_dir`], but passes extra environment variables
/// to each spawned `git config` process. Test seam: lets tests redirect
/// git's global/system scopes via `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM`
/// without mutating process env (racy across parallel tests).
pub(crate) fn effective_hooks_dir_with_env(
    root: &Path,
    envs: &[(String, String)],
) -> Result<EffectiveHooksDir, GitHooksError> {
    if let Some(effective) = first_set_scope(root, envs)? {
        return Ok(effective);
    }
    Ok(EffectiveHooksDir {
        path: root.join(".git/hooks"),
        scope: HooksDirScope::Default,
    })
}

/// Walk config scopes in precedence order and report the first scope
/// where `core.hooksPath` is set; `Ok(None)` when unset everywhere.
fn first_set_scope(
    root: &Path,
    envs: &[(String, String)],
) -> Result<Option<EffectiveHooksDir>, GitHooksError> {
    let scopes = [
        (&["--local"][..], HooksDirScope::Local),
        (&["--global"][..], HooksDirScope::Global),
        (&["--system"][..], HooksDirScope::System),
    ];
    for (scope_args, scope) in scopes {
        if let Some(value) = query_hooks_path(root, scope_args, envs)? {
            return Ok(Some(scope_hooks_dir(value, root, scope)));
        }
    }
    Ok(None)
}

/// Interpret one non-optional `core.hooksPath` value: the empty string
/// disables hooks entirely; relative paths resolve against `root`.
fn scope_hooks_dir(value: String, root: &Path, scope: HooksDirScope) -> EffectiveHooksDir {
    if value.is_empty() {
        return EffectiveHooksDir {
            path: PathBuf::new(),
            scope: HooksDirScope::Disabled,
        };
    }
    let path = if Path::new(&value).is_relative() {
        root.join(value)
    } else {
        PathBuf::from(value)
    };
    EffectiveHooksDir { path, scope }
}

/// Query `core.hooksPath` at one config scope; `Ok(None)` when unset.
///
/// Every exit other than 0 or 1 is an environment failure, never
/// "unset": exit 128 occurs both outside a repository and with a
/// corrupt `.git/config` — walking on to global/system would silently
/// install hooks into the wrong directory (genesis-5sm).
fn query_hooks_path(
    root: &Path,
    scope_args: &[&str],
    envs: &[(String, String)],
) -> Result<Option<String>, GitHooksError> {
    let output = run_hooks_path_query(root, scope_args, envs)?;
    match output.status.code() {
        Some(0) => Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        )),
        // Exit 1 is git's only "key unset at this scope" signal — the
        // sole non-error that lets the scope walk continue.
        Some(1) => Ok(None),
        Some(_) => Err(classify_config_failure(root, &output)),
        // Killed by a signal — not an unset signal either.
        None => Err(classify_config_failure(root, &output)),
    }
}

/// Classify a failed `git config` scope query by its stderr: exit 128
/// outside a repository is [`GitHooksError::NotARepository`], any other
/// failure (corrupt config, …) is [`GitHooksError::ConfigParseFailed`].
fn classify_config_failure(root: &Path, output: &std::process::Output) -> GitHooksError {
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stderr.contains("inside a git repository") || stderr.contains("not a git repository") {
        GitHooksError::NotARepository {
            path: root.to_path_buf(),
        }
    } else {
        GitHooksError::ConfigParseFailed {
            path: root.to_path_buf(),
            message: stderr.trim().to_string(),
        }
    }
}

/// Spawn the scope query, surfacing spawn failures as `Io` at `root`.
fn run_hooks_path_query(
    root: &Path,
    scope_args: &[&str],
    envs: &[(String, String)],
) -> Result<std::process::Output, GitHooksError> {
    hooks_path_command(root, scope_args, envs)
        .output()
        .map_err(|source| GitHooksError::Io {
            path: root.to_path_buf(),
            message: "failed to run `git config core.hooksPath`".to_string(),
            source,
        })
}

/// Build the scope-anchored `git config core.hooksPath` query.
///
/// Anchored at `root`: strip hook-context variables (git exports GIT_DIR
/// to hook processes, and this suite runs inside hooks during testing) so
/// the query reads the repository at `root`, never the ambient GIT_DIR's
/// repository. Same rationale as the fixture guard.
fn hooks_path_command(
    root: &Path,
    scope_args: &[&str],
    envs: &[(String, String)],
) -> std::process::Command {
    let mut cmd = std::process::Command::new("git");
    for var in ["GIT_DIR", "GIT_INDEX_FILE", "GIT_WORK_TREE"] {
        cmd.env_remove(var);
    }
    cmd.args(["-C", &root.to_string_lossy(), "config"])
        .args(scope_args)
        .arg("core.hooksPath");
    for (k, v) in envs {
        cmd.env(k, v);
    }
    cmd
}
