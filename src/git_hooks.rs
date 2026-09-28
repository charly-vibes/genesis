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
//! Boundary note: this module contains only git-hook mechanics — no
//! consuming-tool gate commands. Callers pass their own markers, gate
//! commands, and block contents as parameters; a guard test enforces
//! this boundary.

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
    /// No `lefthook.yml` or `lefthook.yaml` exists — wiring refuses to create
    /// a config from scratch (design D4 non-goal).
    #[error(
        "no supported hook framework config found in {root}: expected lefthook.yml or lefthook.yaml"
    )]
    MissingLefthookConfig {
        /// Repository root that was searched.
        root: PathBuf,
    },
    /// The lefthook config cannot be anchored (e.g. the stage key is quoted
    /// or the structure is unrecognized) — the file is left unmodified.
    #[error(
        "cannot anchor stage '{stage}' in {}: {message} — file left unmodified",
        path.display()
    )]
    UnanchorableLefthookConfig {
        /// The config file that was not modified.
        path: PathBuf,
        /// Stage key that could not be anchored.
        stage: String,
        /// What went wrong while looking for the anchor.
        message: String,
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

/// Identifies which known tool owns a hook file (design D3: ordered sigil
/// table, ported from wai's `hook_owner`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    /// Lefthook-managed hook (sigil: `lefthook`).
    Lefthook,
    /// Husky-managed hook (sigil: `husky`).
    Husky,
    /// bd-managed hook, including bd shims that chain other tools (sigil:
    /// `bd`).
    Bd,
    /// pre-commit framework hook (sigil: `pre-commit`).
    PreCommit,
    /// prek-managed hook (sigil: `prek`).
    Prek,
}

/// The sigil table: more-specific sigils first (design D3). Order matters —
/// a bd hook that chains prek must report [`Owner::Bd`], not
/// [`Owner::Prek`], and a pure prek hook (which always mentions
/// `pre-commit` in its `exec prek run …` line) must report
/// [`Owner::Prek`], not [`Owner::PreCommit`]. Extend by appending entries.
const OWNER_SIGILS: &[(&str, Owner)] = &[
    ("lefthook", Owner::Lefthook),
    ("husky", Owner::Husky),
    ("bd", Owner::Bd),
    ("prek", Owner::Prek),
    ("pre-commit", Owner::PreCommit),
];

fn read_hook_content(root: &Path, hook_name: &HookName) -> Option<String> {
    let path = hook_path(root, hook_name).ok()?;
    if !path.is_file() {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

/// Identify which known tool owns a hook file, or `None` when the hook is
/// missing or carries no known sigil.
///
/// Scans the hook's contents for the ordered sigil table (design D3), so
/// more-specific sigils win over generic ones. Donor: wai
/// `way/hooks.rs::hook_owner`.
pub fn owner(root: &Path, hook_name: &HookName) -> Option<Owner> {
    let content = read_hook_content(root, hook_name)?;
    OWNER_SIGILS
        .iter()
        .find(|(sigil, _)| content.contains(sigil))
        .map(|(_, owner)| *owner)
}

/// Identifies the hook-management framework a repository uses (design D7:
/// the repo is assumed to use at most one framework; reported generically,
/// including husky which has no root config).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Framework {
    /// A `lefthook.yml` or `lefthook.yaml` config exists.
    Lefthook,
    /// A `prek.toml` config exists.
    Prek,
    /// No root config, but hook files carry the husky sigil — detected but
    /// not wirable (wiring is lefthook-only).
    Husky,
    /// No known framework signals.
    None,
}

/// Detect the repository's hook-management framework.
///
/// Precedence on coexisting signals (rare, defensive): Lefthook, then Prek,
/// then Husky (via the [`owner`] sigil table on `pre-commit` / `pre-push`).
/// Donor: espectacular `init.rs::detect_hook_framework`, extended with wai's
/// delegation-aware hook reading.
pub fn framework(root: &Path) -> Framework {
    if root.join("lefthook.yml").exists() || root.join("lefthook.yaml").exists() {
        return Framework::Lefthook;
    }
    if root.join("prek.toml").exists() {
        return Framework::Prek;
    }
    for hook in [HookName::PreCommit, HookName::PrePush] {
        if let Some(Owner::Husky) = owner(root, &hook) {
            return Framework::Husky;
        }
    }
    Framework::None
}

/// Lefthook config wiring on top of [`crate::managed_block`] (design D4:
/// marker-tagged, idempotent blocks; no YAML parser, no bespoke string
/// surgery). Donor: espectacular `init.rs` (rebuilt), wai doctor (wiring
/// check semantics).
pub mod lefthook {
    use super::GitHooksError;
    use crate::managed_block::BlockDef;
    use std::path::{Path, PathBuf};

    /// A lefthook config stage (design D5b: config vocabulary, not hook
    /// file names — kept separate from [`HookName`]).
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Stage {
        /// `pre-commit`
        PreCommit,
        /// `pre-push`
        PrePush,
    }

    impl Stage {
        /// The stage key as it appears at column 0 of a lefthook config.
        pub fn key(&self) -> &'static str {
            match self {
                Stage::PreCommit => "pre-commit",
                Stage::PrePush => "pre-push",
            }
        }
    }

    /// Outcome of [`ensure_wired`].
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum WiredOutcome {
        /// The managed block was injected into the stage section.
        Injected,
        /// The stage section already contained the block — file unchanged.
        AlreadyWired,
    }

    /// Resolve the lefthook config path: `lefthook.yml` wins over
    /// `lefthook.yaml` (design D6); `None` when neither exists.
    fn config_path(root: &Path) -> Option<PathBuf> {
        for name in ["lefthook.yml", "lefthook.yaml"] {
            let path = root.join(name);
            if path.is_file() {
                return Some(path);
            }
        }
        None
    }

    /// True when the line is a column-0 YAML key (starts with a
    /// non-whitespace character other than `#`) — a section boundary.
    fn is_column_zero_key(line: &str) -> bool {
        let first = line.chars().next();
        matches!(first, Some(c) if c != ' ' && c != '\t' && c != '#' && c != '\n')
    }

    /// Find the column-0 stage-key anchor and return its byte offset.
    /// Returns `Err(())` when the stage key appears only in a
    /// non-anchorable form (quoted, indented) — design D6.
    fn find_anchor(content: &str, stage: Stage) -> Result<Option<usize>, ()> {
        let anchor = format!("{}:", stage.key());
        for (offset, line) in content.split_inclusive('\n').scan(0usize, |acc, line| {
            let start = *acc;
            *acc += line.len();
            Some((start, line))
        }) {
            if line.trim_end() == anchor && is_column_zero_key(line) {
                return Ok(Some(offset));
            }
            if line.contains(stage.key()) {
                // Present but not as a column-0 `key:` anchor (quoted,
                // indented, or embedded): unrecognized structure, refuse to
                // guess (design D6).
                return Err(());
            }
        }
        Ok(None)
    }

    /// Extract the stage's section: from the anchor line to the next
    /// column-0 key or EOF (design D5 section semantics).
    fn section(content: &str, anchor_offset: usize) -> &str {
        let rest = &content[anchor_offset..];
        let mut end = rest.len();
        for (offset, line) in rest
            .split_inclusive('\n')
            .scan(0usize, |acc, line| {
                let start = *acc;
                *acc += line.len();
                Some((start, line))
            })
            .skip(1)
        {
            if is_column_zero_key(line) {
                end = offset;
                break;
            }
        }
        &rest[..end]
    }

    /// Inject a managed block into a stage section of the lefthook config,
    /// idempotently.
    ///
    /// The block is inserted directly after the stage key (`pre-commit:` /
    /// `pre-push:` at column 0); a missing stage section is appended.
    /// Errors without modifying the file when no config exists (never
    /// creates one) or when the stage key cannot be anchored (design D6).
    /// Donor: espectacular `init.rs::install_lefthook`, rebuilt on
    /// [`BlockDef`] markers per design D4.
    pub fn ensure_wired(
        root: &Path,
        stage: Stage,
        block: &BlockDef,
        content: &str,
    ) -> Result<WiredOutcome, GitHooksError> {
        let path = config_path(root).ok_or_else(|| GitHooksError::MissingLefthookConfig {
            root: root.to_path_buf(),
        })?;
        let existing = std::fs::read_to_string(&path).map_err(|source| GitHooksError::Io {
            path: path.clone(),
            message: "failed to read lefthook config".to_string(),
            source,
        })?;
        let block_text = format!("{}{}{}", block.start_marker, content, block.end_marker);

        match find_anchor(&existing, stage) {
            Err(()) => Err(GitHooksError::UnanchorableLefthookConfig {
                path,
                stage: stage.key().to_string(),
                message: "stage key is quoted or otherwise not anchorable at column 0".to_string(),
            }),
            Ok(Some(offset)) => {
                // Idempotence is file-level (spec: "the config already
                // contains the block's content"): block markers at column 0
                // are not column-0 keys, so the stage-section scan cannot be
                // relied on to span an injected block.
                if existing.contains(&block_text) || section(&existing, offset).contains(content) {
                    return Ok(WiredOutcome::AlreadyWired);
                }
                // Insert directly after the anchor line (spec: "directly
                // after `pre-commit:`").
                let line_end = existing[offset..]
                    .find('\n')
                    .map_or(existing.len(), |nl| offset + nl + 1);
                let mut updated = String::with_capacity(existing.len() + block_text.len() + 1);
                updated.push_str(&existing[..line_end]);
                if line_end == existing.len() {
                    updated.push('\n'); // anchor was the last line without a newline
                }
                updated.push_str(&block_text);
                updated.push_str(&existing[line_end..]);
                std::fs::write(&path, updated).map_err(|source| GitHooksError::Io {
                    path: path.clone(),
                    message: "failed to write lefthook config".to_string(),
                    source,
                })?;
                Ok(WiredOutcome::Injected)
            }
            Ok(None) => {
                let mut updated = existing.clone();
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(stage.key());
                updated.push_str(":\n");
                updated.push_str(&block_text);
                updated.push('\n');
                std::fs::write(&path, updated).map_err(|source| GitHooksError::Io {
                    path: path.clone(),
                    message: "failed to write lefthook config".to_string(),
                    source,
                })?;
                Ok(WiredOutcome::Injected)
            }
        }
    }

    /// Report whether `command` appears in a stage's section of the
    /// lefthook config, for doctor-style wiring checks (design D5:
    /// stage-scoped scan — a pre-push command must not satisfy a
    /// pre-commit check). Returns `false` when the command is absent, the
    /// stage section is missing, or no config exists.
    pub fn is_wired(root: &Path, stage: Stage, command: &str) -> bool {
        let Some(path) = config_path(root) else {
            return false;
        };
        let Ok(existing) = std::fs::read_to_string(&path) else {
            return false;
        };
        match find_anchor(&existing, stage) {
            Ok(Some(offset)) => section(&existing, offset).contains(command),
            _ => false,
        }
    }
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

    // -- Owner detection --------------------------------------------------

    fn write_hook(fixture: &Fixture, hook: HookName, content: &str) {
        let path = fixture.root().join(".git/hooks").join(hook.file_name());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn owner_detects_each_sigil_in_table_order() {
        let cases: &[(&str, Owner)] = &[
            ("#!/bin/sh\n# managed by lefthook\n", Owner::Lefthook),
            (
                "#!/bin/sh\n. \"$(dirname -- \"$0\")\"/_/husky.sh\n",
                Owner::Husky,
            ),
            (
                "#!/bin/sh\nexec bd hooks run pre-commit \"$@\"\n",
                Owner::Bd,
            ),
            (
                "#!/bin/sh\n# installed by pre-commit framework\n", // generic
                Owner::PreCommit,
            ),
            (
                "#!/bin/sh\n# managed by prek\nexec prek run pre-commit\n",
                Owner::Prek,
            ),
        ];
        for (content, expected) in cases {
            let fixture = install_fixture();
            write_hook(&fixture, HookName::PreCommit, content);
            let got = owner(fixture.root(), &HookName::PreCommit)
                .unwrap_or_else(|| panic!("expected {expected:?} for {content:?}"));
            assert_eq!(got, *expected, "content: {content:?}");
        }
    }

    #[test]
    fn owner_most_specific_sigil_wins_bd_shim_chaining_prek() {
        // wai test: a bd hook that chains prek must report bd, not prek.
        let fixture = install_fixture();
        write_hook(
            &fixture,
            HookName::PreCommit,
            "#!/usr/bin/env sh\n# bd-shim v2\n# chains prek\nexec prek run pre-commit\n",
        );
        assert_eq!(owner(fixture.root(), &HookName::PreCommit), Some(Owner::Bd));
    }

    #[test]
    fn owner_none_for_missing_or_unknown_hook() {
        let fixture = install_fixture();
        assert_eq!(owner(fixture.root(), &HookName::PreCommit), None);
        write_hook(&fixture, HookName::PreCommit, "#!/bin/sh\necho custom\n");
        assert_eq!(owner(fixture.root(), &HookName::PreCommit), None);
    }

    #[test]
    fn owner_resolves_hooks_dir() {
        // Detection must honor core.hooksPath like the rest of the module.
        let fixture = match Fixture::new().with_git_init().build() {
            Ok(f) => f,
            Err(e) => panic!("fixture build failed: {e}"),
        };
        fixture
            .run(&["git", "config", "--local", "core.hooksPath", ".githooks"])
            .unwrap();
        let delegated = fixture.root().join(".githooks");
        std::fs::create_dir_all(&delegated).unwrap();
        std::fs::write(delegated.join("pre-commit"), "# managed by lefthook\n").unwrap();
        assert_eq!(
            owner(fixture.root(), &HookName::PreCommit),
            Some(Owner::Lefthook)
        );
    }

    // -- Framework detection ----------------------------------------------

    #[test]
    fn framework_lefthook_config_present() {
        for name in ["lefthook.yml", "lefthook.yaml"] {
            let fixture = install_fixture();
            std::fs::write(fixture.root().join(name), "pre-commit:\n  commands:\n").unwrap();
            assert_eq!(framework(fixture.root()), Framework::Lefthook, "{name}");
        }
    }

    #[test]
    fn framework_prek_config_present() {
        let fixture = install_fixture();
        std::fs::write(fixture.root().join("prek.toml"), "").unwrap();
        assert_eq!(framework(fixture.root()), Framework::Prek);
    }

    #[test]
    fn framework_husky_via_hook_sigil_without_root_config() {
        let fixture = install_fixture();
        write_hook(
            &fixture,
            HookName::PreCommit,
            "#!/bin/sh\n. \"$(dirname -- \"$0\")\"/_/husky.sh\n",
        );
        assert_eq!(framework(fixture.root()), Framework::Husky);
    }

    #[test]
    fn framework_none_without_signals() {
        let fixture = install_fixture();
        assert_eq!(framework(fixture.root()), Framework::None);
    }

    #[test]
    fn framework_precedence_lefthook_then_prek_then_husky() {
        // Lefthook beats everything.
        let fixture = install_fixture();
        std::fs::write(fixture.root().join("lefthook.yml"), "").unwrap();
        std::fs::write(fixture.root().join("prek.toml"), "").unwrap();
        write_hook(&fixture, HookName::PreCommit, "# husky\n");
        assert_eq!(framework(fixture.root()), Framework::Lefthook);

        // Prek beats husky sigils.
        let fixture = install_fixture();
        std::fs::write(fixture.root().join("prek.toml"), "").unwrap();
        write_hook(&fixture, HookName::PreCommit, "# husky\n");
        assert_eq!(framework(fixture.root()), Framework::Prek);
    }

    // -- Lefthook wiring (tasks 4.1-4.5) ----------------------------------

    mod wiring_tests {
        use super::*;
        use crate::managed_block::BlockDef;

        fn ah_block() -> BlockDef {
            BlockDef::new("AH")
        }

        fn ah_content() -> &'static str {
            "  commands:\n    ah:\n      run: ah check\n"
        }

        fn basic_config() -> &'static str {
            "pre-commit:\n  commands:\n    lint:\n      run: lint\npre-push:\n  commands:\n    test:\n      run: test\n"
        }

        fn config_fixture(contents: &str) -> Fixture {
            let fixture = match Fixture::new().build() {
                Ok(f) => f,
                Err(e) => panic!("fixture build failed: {e}"),
            };
            std::fs::write(fixture.root().join("lefthook.yml"), contents).unwrap();
            fixture
        }

        #[test]
        fn ensure_wired_inserts_block_directly_after_stage_key() {
            let fixture = config_fixture(basic_config());
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            let block_text = format!(
                "{}{}{}",
                ah_block().start_marker,
                ah_content(),
                ah_block().end_marker
            );
            let expected = format!(
                "pre-commit:\n{block_text}{}",
                &basic_config()["pre-commit:\n".len()..]
            );
            assert_eq!(after, expected);
        }

        #[test]
        fn ensure_wired_creates_missing_stage_section() {
            // Config has only a pre-commit section; wiring pre-push must
            // append the missing stage section containing the block.
            let config = "pre-commit:\n  commands:\n    lint:\n      run: lint\n";
            let fixture = match Fixture::new().build() {
                Ok(f) => f,
                Err(e) => panic!("fixture build failed: {e}"),
            };
            std::fs::write(fixture.root().join("lefthook.yml"), config).unwrap();
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PrePush,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            let block_text = format!(
                "{}{}{}",
                ah_block().start_marker,
                ah_content(),
                ah_block().end_marker
            );
            assert!(after.starts_with(config));
            assert!(after.contains(&format!("\npre-push:\n{block_text}\n")));
        }

        #[test]
        fn ensure_wired_is_idempotent_when_content_present() {
            let fixture = config_fixture(basic_config());
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let once = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let twice = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            assert_eq!(once, twice);
            assert_eq!(twice.matches("AH:START").count(), 1);
        }

        #[test]
        fn ensure_wired_errors_on_missing_config_and_never_creates_one() {
            let fixture = match Fixture::new().build() {
                Ok(f) => f,
                Err(e) => panic!("fixture build failed: {e}"),
            };
            let err = lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap_err();
            let msg = err.to_string();
            assert!(
                msg.contains("no supported hook framework config"),
                "error should explain: {msg}"
            );
            assert!(!fixture.root().join("lefthook.yml").exists());
            assert!(!fixture.root().join("lefthook.yaml").exists());
        }

        #[test]
        fn ensure_wired_prefers_yml_when_both_exist() {
            let fixture = match Fixture::new().build() {
                Ok(f) => f,
                Err(e) => panic!("fixture build failed: {e}"),
            };
            std::fs::write(fixture.root().join("lefthook.yml"), basic_config()).unwrap();
            std::fs::write(fixture.root().join("lefthook.yaml"), "other: file\n").unwrap();
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let yml = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            let yaml = std::fs::read_to_string(fixture.root().join("lefthook.yaml")).unwrap();
            assert!(yml.contains("AH:START"), ".yml should be modified");
            assert_eq!(yaml, "other: file\n", ".yaml must be ignored (D6)");
        }

        #[test]
        fn ensure_wired_errors_unmodified_on_quoted_stage_key() {
            let quoted = "\"pre-push\":\n  commands:\n    test:\n      run: test\n";
            let fixture = config_fixture(quoted);
            let err = lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PrePush,
                &ah_block(),
                ah_content(),
            )
            .unwrap_err();
            assert!(
                matches!(err, GitHooksError::UnanchorableLefthookConfig { .. }),
                "got: {err}"
            );
            let after = std::fs::read_to_string(fixture.root().join("lefthook.yml")).unwrap();
            assert_eq!(after, quoted, "config must not be modified");
        }

        #[test]
        fn injected_block_is_recognized_by_managed_block_reader() {
            let fixture = config_fixture(basic_config());
            lefthook::ensure_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                &ah_block(),
                ah_content(),
            )
            .unwrap();
            let mut reg = crate::managed_block::BlockRegistry::new();
            reg.register(ah_block());
            let injector = crate::managed_block::BlockInjector::new(reg);
            let path = fixture.root().join("lefthook.yml");
            assert!(injector.has_block(&path, "AH"));
            let read = injector.read_block(&path, "AH").unwrap();
            assert!(read.contains(ah_block().start_marker.as_str()));
            assert!(read.contains("run: ah check"));
        }

        #[test]
        fn is_wired_true_when_command_in_target_stage() {
            let fixture = config_fixture(
                "pre-commit:\n  commands:\n    ah:\n      run: ah check\npre-push:\n  commands:\n    test:\n      run: test\n",
            );
            assert!(lefthook::is_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                "ah check"
            ));
            assert!(!lefthook::is_wired(
                fixture.root(),
                lefthook::Stage::PrePush,
                "ah check"
            ));
        }

        #[test]
        fn is_wired_false_when_stage_missing_or_no_config() {
            let fixture = config_fixture(basic_config());
            assert!(!lefthook::is_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                "ah check"
            ));
            let empty = match Fixture::new().build() {
                Ok(f) => f,
                Err(e) => panic!("fixture build failed: {e}"),
            };
            assert!(!lefthook::is_wired(
                empty.root(),
                lefthook::Stage::PreCommit,
                "ah check"
            ));
        }

        #[test]
        fn is_wired_command_in_wrong_stage_is_not_wired() {
            let fixture = config_fixture("pre-push:\n  commands:\n    ah:\n      run: ah check\n");
            assert!(lefthook::is_wired(
                fixture.root(),
                lefthook::Stage::PrePush,
                "ah check"
            ));
            assert!(!lefthook::is_wired(
                fixture.root(),
                lefthook::Stage::PreCommit,
                "ah check"
            ));
        }

        /// Guard (spec: "Tool-specific gates stay in tools"): the
        /// non-test portion of this module must not hardcode consuming-tool
        /// gate commands. Test fixtures may use arbitrary gate strings, so
        /// only the source up to this test module is scanned.
        #[test]
        fn module_source_has_no_tool_gate_strings() {
            const SOURCE: &str = include_str!("git_hooks.rs");
            let production = SOURCE
                .split_once("#[cfg(test)]")
                .expect("test module marker must exist")
                .0;
            for gate in [
                "ah check",
                "pretender check",
                "testaruda select",
                "just check-claims",
            ] {
                assert!(
                    !production.contains(gate),
                    "git_hooks module must not hardcode the consuming-tool gate {gate:?}"
                );
            }
        }
    }
}
