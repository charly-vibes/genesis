//! Hook install/uninstall with ownership markers (design D1/D6).

use std::path::{Path, PathBuf};

use super::GitHooksError;
use super::HookName;
use super::hook::hook_path_with_env;

/// Ownership state of a hook file on disk.
enum HookOwnership {
    /// No file at the hook path.
    Missing,
    /// File exists and carries the marker.
    Owned,
    /// File exists but has no marker — not ours to touch (0-byte files are
    /// foreign per design D6).
    Foreign,
}

/// Read a hook file and classify its ownership by `marker`.
fn read_hook_ownership(path: &Path, marker: &str) -> Result<HookOwnership, GitHooksError> {
    if !path.exists() {
        return Ok(HookOwnership::Missing);
    }
    let contents = std::fs::read_to_string(path).map_err(|source| GitHooksError::Io {
        path: path.to_path_buf(),
        message: "failed to read hook".to_string(),
        source,
    })?;
    let owned = contents.contains(marker);
    if owned {
        Ok(HookOwnership::Owned)
    } else {
        Ok(HookOwnership::Foreign)
    }
}

/// Insert the marker after the script's shebang line (pretender's
/// convention) so the file stays executable-compatible; prepend when the
/// script has no shebang.
fn apply_marker(script: &str, marker: &str) -> String {
    match script
        .strip_prefix("#!")
        .and_then(|rest| rest.split_once('\n'))
    {
        Some((shebang, body)) => format!("#!{shebang}\n{marker}\n{body}"),
        None => format!("{marker}\n{script}"),
    }
}

fn set_executable(path: &Path) -> Result<(), GitHooksError> {
    #[cfg(unix)]
    set_executable_unix(path)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Mode 0755: git refuses to run hooks that are not executable.
#[cfg(unix)]
fn set_executable_unix(path: &Path) -> Result<(), GitHooksError> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)
        .map_err(|source| GitHooksError::Io {
            path: path.to_path_buf(),
            message: "failed to stat hook".to_string(),
            source,
        })?
        .permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).map_err(|source| GitHooksError::Io {
        path: path.to_path_buf(),
        message: "failed to chmod hook".to_string(),
        source,
    })?;
    Ok(())
}

fn io_err(path: &Path, message: &str, source: std::io::Error) -> GitHooksError {
    GitHooksError::Io {
        path: path.to_path_buf(),
        message: message.to_string(),
        source,
    }
}

/// Write an executable hook script into the repository's hooks directory,
/// identified by an ownership marker (design D1).
///
/// The marker is inserted after the script's shebang line (pretender's
/// convention) so the file stays executable-compatible. Refuses to modify
/// an existing hook that lacks the marker, including empty (0-byte) files
/// (design D6).
pub fn install(
    root: &Path,
    hook_name: HookName,
    marker: &str,
    script: &str,
) -> Result<PathBuf, GitHooksError> {
    install_with_env(root, hook_name, marker, script, &[])
}

/// Like [`install`], but passes extra environment variables to any spawned
/// `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
pub(crate) fn install_with_env(
    root: &Path,
    hook_name: HookName,
    marker: &str,
    script: &str,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    let path = hook_path_with_env(root, &hook_name, envs)?;
    ensure_writable(&path, marker)?;
    create_hook_parent_dir(&path)?;
    std::fs::write(&path, apply_marker(script, marker))
        .map_err(|source| io_err(&path, "failed to write hook", source))?;
    set_executable(&path)?;
    Ok(path)
}

/// Create the hooks directory when missing (e.g. fresh `.git/hooks`).
fn create_hook_parent_dir(path: &Path) -> Result<(), GitHooksError> {
    let parent = path.parent().ok_or_else(|| {
        io_err(
            path,
            "invalid hook path (no parent directory)",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent"),
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|source| io_err(parent, "failed to create hook dir", source))
}

/// Refuse to overwrite a foreign hook (design D6: empty files are foreign).
fn ensure_writable(path: &Path, marker: &str) -> Result<(), GitHooksError> {
    match read_hook_ownership(path, marker)? {
        HookOwnership::Foreign => Err(GitHooksError::ForeignHook {
            path: path.to_path_buf(),
            action: "overwrite",
        }),
        HookOwnership::Missing | HookOwnership::Owned => Ok(()),
    }
}

/// Remove a hook only when the ownership marker matches (design D1).
///
/// No-op when the hook file does not exist; refuses to remove a foreign
/// hook.
pub fn uninstall(root: &Path, hook_name: HookName, marker: &str) -> Result<PathBuf, GitHooksError> {
    uninstall_with_env(root, hook_name, marker, &[])
}

/// Like [`uninstall`], but passes extra environment variables to any
/// spawned `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
pub(crate) fn uninstall_with_env(
    root: &Path,
    hook_name: HookName,
    marker: &str,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    let path = hook_path_with_env(root, &hook_name, envs)?;
    match read_hook_ownership(&path, marker)? {
        HookOwnership::Missing => Ok(path),
        HookOwnership::Foreign => Err(GitHooksError::ForeignHook {
            path,
            action: "remove",
        }),
        HookOwnership::Owned => {
            std::fs::remove_file(&path)
                .map_err(|source| io_err(&path, "failed to remove hook", source))?;
            Ok(path)
        }
    }
}
