//! Git hook primitives shared across the suite.
//!
//! Consolidates overlapping git-hook mechanics from three donors:
//!
//! - pretender (`src/main.rs`): install/uninstall with ownership marker,
//!   refuse-foreign-hook guard, `repo_root()` parent walk.
//! - wai (`src/commands/way/hooks.rs`): `core.hooksPath` resolution,
//!   hook-read helpers.
//! - espectacular (`src/init.rs`): lefthook framework detection and
//!   managed-block injection (rebuilt on [`crate::managed_block`]).
//!
//! Boundary note: this module contains no consuming-tool gate commands
//! (e.g. `ah check`, `pretender check`) — those stay in the tools.

use std::path::{Path, PathBuf};

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
}

/// Locate the enclosing git repository by walking parent directories
/// until a `.git` entry is found.
///
/// Walks up from the current directory. Donor: pretender `repo_root()`.
pub fn repo_root() -> Result<PathBuf, GitHooksError> {
    let start = std::env::current_dir().map_err(|source| GitHooksError::Io {
        path: PathBuf::from("."),
        message: "failed to get current directory".to_string(),
        source,
    })?;
    repo_root_from(&start)
}

/// Like [`repo_root()`], but walks up from an explicit starting directory.
///
/// Exposed for tests and callers that already hold a working directory.
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitHooksError> {
    let mut current = Some(start);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return Ok(dir.to_path_buf());
        }
        current = dir.parent();
    }
    Err(GitHooksError::NotInRepo {
        start: start.to_path_buf(),
    })
}

/// Resolve the repository's hook directory.
///
/// Returns the local `core.hooksPath` when set (relative paths are
/// resolved against `root`, matching git's semantics), else the default
/// `.git/hooks`. Donor: wai `git_core_hooks_path()`.
pub fn resolve_hooks_dir(root: &Path) -> Result<PathBuf, GitHooksError> {
    let output = std::process::Command::new("git")
        .args([
            "-C",
            &root.to_string_lossy(),
            "config",
            "--local",
            "core.hooksPath",
        ])
        .output()
        .map_err(|source| GitHooksError::Io {
            path: root.to_path_buf(),
            message: "failed to run `git config --local core.hooksPath`".to_string(),
            source,
        })?;
    // Non-zero exit means the config is unset — fall back to the default.
    let hooks_path = if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if val.is_empty() { None } else { Some(val) }
    } else {
        None
    };
    match hooks_path {
        // Relative paths resolve against the repo root (git's own
        // semantics for core.hooksPath).
        Some(p) if Path::new(&p).is_relative() => Ok(root.join(p)),
        Some(p) => Ok(PathBuf::from(p)),
        None => Ok(root.join(".git/hooks")),
    }
}

/// Identifies a hook by file name (design D5b: hook names, not lefthook
/// config stages).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookName {
    /// `pre-commit`
    PreCommit,
    /// `pre-push`
    PrePush,
    /// Escape hatch for hook file names outside the donor set (design
    /// trade-off note: only donor names get dedicated variants).
    Other(String),
}

impl HookName {
    /// The hook file name git executes inside the hooks directory.
    pub fn file_name(&self) -> &str {
        match self {
            HookName::PreCommit => "pre-commit",
            HookName::PrePush => "pre-push",
            HookName::Other(name) => name,
        }
    }
}

/// Path of a hook file, resolved through [`resolve_hooks_dir`] (design D2:
/// `core.hooksPath` is honored everywhere).
fn hook_path(root: &Path, hook_name: &HookName) -> Result<PathBuf, GitHooksError> {
    Ok(resolve_hooks_dir(root)?.join(hook_name.file_name()))
}

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
    {
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
    }
    #[cfg(not(unix))]
    let _ = path;
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
    let path = hook_path(root, &hook_name)?;
    match read_hook_ownership(&path, marker)? {
        HookOwnership::Foreign => {
            return Err(GitHooksError::ForeignHook {
                path,
                action: "overwrite",
            });
        }
        HookOwnership::Missing | HookOwnership::Owned => {}
    }
    let parent = path.parent().ok_or_else(|| {
        io_err(
            &path,
            "invalid hook path (no parent directory)",
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "no parent"),
        )
    })?;
    std::fs::create_dir_all(parent)
        .map_err(|source| io_err(parent, "failed to create hook dir", source))?;
    std::fs::write(&path, apply_marker(script, marker))
        .map_err(|source| io_err(&path, "failed to write hook", source))?;
    set_executable(&path)?;
    Ok(path)
}

/// Remove a hook only when the ownership marker matches (design D1).
///
/// No-op when the hook file does not exist; refuses to remove a foreign
/// hook.
pub fn uninstall(root: &Path, hook_name: HookName, marker: &str) -> Result<PathBuf, GitHooksError> {
    let path = hook_path(root, &hook_name)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    // -- Repository root discovery ---------------------------------------

    #[test]
    fn repo_root_finds_root_from_nested_dir() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let nested = fixture.path("a/b/c");
        std::fs::create_dir_all(&nested).unwrap();
        let found = repo_root_from(&nested).unwrap();
        assert_eq!(found, fixture.root());
    }

    #[test]
    fn repo_root_from_the_root_itself_works() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let found = repo_root_from(fixture.root()).unwrap();
        assert_eq!(found, fixture.root());
    }

    #[test]
    fn repo_root_errors_outside_a_repository() {
        let fixture = match Fixture::new().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let err = repo_root_from(fixture.root()).unwrap_err();
        assert!(matches!(err, GitHooksError::NotInRepo { .. }));
        let msg = err.to_string();
        assert!(
            msg.contains("not inside a git repository"),
            "error should name the problem: {msg}"
        );
    }

    // -- Hooks directory resolution --------------------------------------

    #[test]
    fn resolve_hooks_dir_defaults_to_git_hooks() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let resolved = resolve_hooks_dir(fixture.root()).unwrap();
        assert_eq!(resolved, fixture.root().join(".git/hooks"));
    }

    #[test]
    fn resolve_hooks_dir_honors_local_core_hooks_path() {
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        let out = fixture
            .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
            .unwrap();
        assert!(out.success(), "git config failed: {}", out.stderr);
        let resolved = resolve_hooks_dir(fixture.root()).unwrap();
        assert_eq!(resolved, fixture.root().join(".githooks"));
    }

    // -- Hook file names -------------------------------------------------

    #[test]
    fn hook_name_file_name_mapping() {
        assert_eq!(HookName::PreCommit.file_name(), "pre-commit");
        assert_eq!(HookName::PrePush.file_name(), "pre-push");
        assert_eq!(
            HookName::Other("commit-msg".to_string()).file_name(),
            "commit-msg"
        );
    }

    // -- Install ---------------------------------------------------------

    fn install_fixture() -> Fixture {
        match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        }
    }

    const TEST_MARKER: &str = "# Installed by test-tool.";
    const TEST_SCRIPT: &str = "#!/usr/bin/env sh\nexec true\n";

    #[test]
    fn install_writes_executable_hook_with_marker_and_script() {
        let fixture = install_fixture();
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        let hook_path = fixture.root().join(".git/hooks/pre-commit");
        let contents = std::fs::read_to_string(&hook_path).unwrap();
        assert!(contents.contains(TEST_MARKER), "marker missing: {contents}");
        assert!(contents.contains("exec true"), "script missing: {contents}");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&hook_path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o755, "hook must be executable 0755");
        }
    }

    #[test]
    fn install_creates_hooks_dir_on_demand() {
        let fixture = install_fixture();
        std::fs::remove_dir_all(fixture.root().join(".git/hooks")).unwrap();
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        assert!(fixture.root().join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn install_respects_core_hooks_path() {
        let fixture = install_fixture();
        let out = fixture
            .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
            .unwrap();
        assert!(out.success(), "git config failed: {}", out.stderr);
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        assert!(fixture.root().join(".githooks/pre-commit").exists());
        assert!(!fixture.root().join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn install_refuses_foreign_hook_unmodified() {
        let fixture = install_fixture();
        let hook_path = fixture.root().join(".git/hooks/pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\necho mine\n").unwrap();
        let err = install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::ForeignHook { .. }),
            "got: {err}"
        );
        let msg = err.to_string();
        assert!(msg.contains("refusing"), "error should explain: {msg}");
        assert_eq!(
            std::fs::read_to_string(&hook_path).unwrap(),
            "#!/bin/sh\necho mine\n"
        );
    }

    #[test]
    fn install_refuses_empty_hook_file() {
        // Design D6: a 0-byte hook file is foreign — refuse rather than
        // silently overwrite a file the user created.
        let fixture = install_fixture();
        std::fs::write(fixture.root().join(".git/hooks/pre-commit"), "").unwrap();
        let err = install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap_err();
        assert!(
            matches!(err, GitHooksError::ForeignHook { .. }),
            "got: {err}"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).unwrap(),
            ""
        );
    }

    #[test]
    fn install_is_idempotent_for_own_hooks() {
        let fixture = install_fixture();
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        let v2 = "#!/usr/bin/env sh\nexec false\n";
        install(fixture.root(), HookName::PreCommit, TEST_MARKER, v2).unwrap();
        let contents =
            std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).unwrap();
        assert!(contents.contains(TEST_MARKER));
        assert!(contents.contains("exec false"));
    }

    #[test]
    fn install_marker_follows_shebang() {
        // Pretender's convention: shebang stays line 1, marker after it.
        let fixture = install_fixture();
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        let contents =
            std::fs::read_to_string(fixture.root().join(".git/hooks/pre-commit")).unwrap();
        let mut lines = contents.lines();
        assert_eq!(lines.next(), Some("#!/usr/bin/env sh"));
        assert_eq!(lines.next(), Some(TEST_MARKER));
    }

    // -- Uninstall -------------------------------------------------------

    #[test]
    fn uninstall_removes_owned_hook() {
        let fixture = install_fixture();
        install(
            fixture.root(),
            HookName::PreCommit,
            TEST_MARKER,
            TEST_SCRIPT,
        )
        .unwrap();
        uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).unwrap();
        assert!(!fixture.root().join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn uninstall_is_noop_for_missing_hook() {
        let fixture = install_fixture();
        uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).unwrap();
    }

    #[test]
    fn uninstall_refuses_foreign_hook() {
        let fixture = install_fixture();
        let hook_path = fixture.root().join(".git/hooks/pre-commit");
        std::fs::write(&hook_path, "#!/bin/sh\necho mine\n").unwrap();
        let err = uninstall(fixture.root(), HookName::PreCommit, TEST_MARKER).unwrap_err();
        assert!(
            matches!(err, GitHooksError::ForeignHook { .. }),
            "got: {err}"
        );
        assert!(hook_path.exists());
    }
}
