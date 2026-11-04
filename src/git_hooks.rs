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
    /// A managed-block marker is present but its pair is not — the
    /// config is in an unknown state and wiring refuses to guess; the
    /// file is left unmodified.
    #[error(
        "unbalanced managed-block markers in {}: found one of the '{start}'/'{end}' pair but not both — fix or remove the markers manually",
        path.display()
    )]
    UnbalancedLefthookMarkers {
        /// Config file with the unbalanced markers.
        path: PathBuf,
        /// The start marker that was searched.
        start: String,
        /// The end marker that was searched.
        end: String,
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
///
/// Alias delegating to [`crate::git::repo_root_from`] — the walk logic
/// moved into the shared `git` module (genesis-tpf); behavior is
/// unchanged, and `git::GitError::NotInRepo`/`Io` map back onto the
/// matching [`GitHooksError`] variants.
pub fn repo_root_from(start: &Path) -> Result<PathBuf, GitHooksError> {
    crate::git::repo_root_from(start).map_err(map_root_error)
}

/// Map a [`crate::git::GitError`] from the root walk onto the
/// [`GitHooksError`] variants callers of this module expect.
fn map_root_error(error: crate::git::GitError) -> GitHooksError {
    match error {
        crate::git::GitError::NotInRepo { start } => GitHooksError::NotInRepo { start },
        crate::git::GitError::Io {
            path,
            message,
            source,
        } => GitHooksError::Io {
            path,
            message,
            source,
        },
        // The root walk never spawns git, so `Git`/`Spawn` are unreachable;
        // still map them exhaustively onto `Io` for future-proofing.
        other => GitHooksError::Io {
            path: PathBuf::from("."),
            message: other.to_string(),
            source: std::io::Error::other(other.to_string()),
        },
    }
}

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
fn resolve_hooks_dir_with_env(
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
fn effective_hooks_dir_with_env(
    root: &Path,
    envs: &[(String, String)],
) -> Result<EffectiveHooksDir, GitHooksError> {
    let scopes = [
        (&["--local"][..], HooksDirScope::Local),
        (&["--global"][..], HooksDirScope::Global),
        (&["--system"][..], HooksDirScope::System),
    ];
    for (scope_args, scope) in scopes {
        if let Some(value) = query_hooks_path(root, scope_args, envs)? {
            if value.is_empty() {
                return Ok(EffectiveHooksDir {
                    path: PathBuf::new(),
                    scope: HooksDirScope::Disabled,
                });
            }
            let path = if Path::new(&value).is_relative() {
                root.join(value)
            } else {
                PathBuf::from(value)
            };
            return Ok(EffectiveHooksDir { path, scope });
        }
    }
    Ok(EffectiveHooksDir {
        path: root.join(".git/hooks"),
        scope: HooksDirScope::Default,
    })
}

/// Query `core.hooksPath` at one config scope; `Ok(None)` when unset.
fn query_hooks_path(
    root: &Path,
    scope_args: &[&str],
    envs: &[(String, String)],
) -> Result<Option<String>, GitHooksError> {
    // Anchored at `root`: strip hook-context variables (git exports
    // GIT_DIR to hook processes, and this suite runs inside hooks during
    // testing) so the query reads the repository at `root`, never the
    // ambient GIT_DIR's repository. Same rationale as the fixture guard.
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
    let output = cmd.output().map_err(|source| GitHooksError::Io {
        path: root.to_path_buf(),
        message: "failed to run `git config core.hooksPath`".to_string(),
        source,
    })?;
    // Non-zero exit means the config is unset at this scope.
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
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
fn hook_path_with_env(
    root: &Path,
    hook_name: &HookName,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    Ok(resolve_hooks_dir_with_env(root, envs)?.join(hook_name.file_name()))
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

fn read_hook_content(
    root: &Path,
    hook_name: &HookName,
    envs: &[(String, String)],
) -> Option<String> {
    let path = hook_path_with_env(root, hook_name, envs).ok()?;
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
    owner_with_env(root, hook_name, &[])
}

/// Like [`owner`], but passes extra environment variables to any spawned
/// `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
fn owner_with_env(root: &Path, hook_name: &HookName, envs: &[(String, String)]) -> Option<Owner> {
    let content = read_hook_content(root, hook_name, envs)?;
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
    framework_with_env(root, &[])
}

/// Like [`framework`], but passes extra environment variables to any
/// spawned `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
fn framework_with_env(root: &Path, envs: &[(String, String)]) -> Framework {
    if root.join("lefthook.yml").exists() || root.join("lefthook.yaml").exists() {
        return Framework::Lefthook;
    }
    if root.join("prek.toml").exists() {
        return Framework::Prek;
    }
    for hook in [HookName::PreCommit, HookName::PrePush] {
        if let Some(Owner::Husky) = owner_with_env(root, &hook, envs) {
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

    /// Indent of the stage section's first child line (skips the anchor
    /// line itself, blank lines and comments). `None` when the stage is
    /// empty — used to classify the wiring case.
    fn children_indent(section: &str) -> Option<usize> {
        for line in section.split_inclusive('\n').skip(1) {
            let trimmed = line.trim_end();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            return Some(line.len() - line.trim_start().len());
        }
        None
    }

    /// Byte offset (within `section`) and indent of a `commands:` key
    /// line, at any indent — the caller compares against
    /// [`children_indent`] to classify: at-children → insert inside the
    /// mapping, elsewhere → refuse, absent → wrapper path.
    fn find_commands_key(section: &str) -> Option<(usize, usize)> {
        let mut offset = 0usize;
        for line in section.split_inclusive('\n') {
            let start = offset;
            offset += line.len();
            if line.trim() == "commands:" {
                return Some((start, line.len() - line.trim_start().len()));
            }
        }
        None
    }

    /// Find the column-0 stage-key anchor and return its byte offset.
    /// Returns `Err(())` when the stage key appears only in a
    /// non-anchorable form (quoted, indented) — design D6. Comment
    /// lines (trimmed starts with `#`) are skipped entirely: a mention
    /// of the stage name in a comment neither anchors nor refuses
    /// (GH#28 item 1).
    fn find_anchor(content: &str, stage: Stage) -> Result<Option<usize>, ()> {
        let anchor = format!("{}:", stage.key());
        for (offset, line) in content.split_inclusive('\n').scan(0usize, |acc, line| {
            let start = *acc;
            *acc += line.len();
            Some((start, line))
        }) {
            if line.trim_start().starts_with('#') {
                // Comment: mention of the stage name here is prose, not
                // structure — skip before both the anchor and refusal
                // checks.
                continue;
            }
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
    /// The block is nested inside the stage's `commands:` mapping — inserted
    /// directly after the `commands:` line, with `  commands:` emitted after
    /// the stage anchor (`pre-commit:` / `pre-push:` at column 0) when the
    /// stage lacks one. Caller content is the entries belonging inside
    /// `commands:` (inserted verbatim, no re-indentation); a stage-level
    /// key is silently ignored by lefthook at runtime (genesis-r99). A
    /// missing stage section is appended (with `commands:`). Errors without
    /// modifying the file when no config exists (never creates one) or when
    /// the stage key cannot be anchored (design D6).
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
                // genesis-r99: lefthook only honors commands declared under
                // the stage's `commands:` mapping — a stage-level key is
                // silently ignored at runtime (validate rejects it, commit
                // skips it). Nest the block inside `commands:`, emitting
                // the mapping after the stage anchor when the stage lacks
                // one. Caller content is inserted verbatim: it is written
                // for the `commands:` child level.
                let stage_section = section(&existing, offset);
                let mut commands_end: Option<usize> = None;
                let mut scanned = 0usize;
                for (i, line) in stage_section.split_inclusive('\n').enumerate() {
                    if i > 0 {
                        let trimmed = line.trim_end();
                        if trimmed.trim() == "commands:" && !trimmed.starts_with('#') {
                            commands_end = Some(scanned + line.len());
                            break;
                        }
                    }
                    scanned += line.len();
                }
                // Anchor line end, as a byte offset into `existing`.
                let anchor_line_end = existing[offset..]
                    .find('\n')
                    .map_or(existing.len(), |nl| offset + nl + 1);
                let (insert_at, prefix) = match commands_end {
                    // `commands_end` is relative to the stage section:
                    // re-base it onto the full config before splicing.
                    Some(end) => (offset + end, ""),
                    None => (anchor_line_end, "  commands:\n"),
                };
                let mut updated =
                    String::with_capacity(existing.len() + block_text.len() + prefix.len() + 2);
                updated.push_str(&existing[..insert_at]);
                if insert_at == existing.len() {
                    updated.push('\n'); // anchor/commands was the last line without a newline
                }
                updated.push_str(prefix);
                updated.push_str(&block_text);
                // The block always ends on its own line: without this, a
                // caller content ending in `\n` leaves the END marker
                // glued onto the next existing line — with comment-
                // prefixed markers that line becomes a YAML comment and
                // the following key is silently deleted (genesis-au8).
                if !block_text.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(&existing[insert_at..]);
                std::fs::write(&path, updated).map_err(|source| GitHooksError::Io {
                    path: path.clone(),
                    message: "failed to write lefthook config".to_string(),
                    source,
                })?;
                Ok(WiredOutcome::Injected)
            }
            Ok(None) => {
                // genesis-r99: appended stages also nest the block inside
                // `commands:` so the wired command is a real lefthook
                // command, not a silently-ignored stage-level key.
                let mut updated = existing.clone();
                if !updated.ends_with('\n') {
                    updated.push('\n');
                }
                updated.push_str(stage.key());
                updated.push_str(":\n  commands:\n");
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

    /// Wire a single command into a stage's `commands:` mapping of the
    /// lefthook config, idempotently (genesis-au8).
    ///
    /// Two-case anchor, lifted from specodelic `src/hooks.rs` (donor for
    /// [`ensure_wired`]'s sibling contract):
    /// - stage section has `commands:` at the children indent → the
    ///   marker-guarded entry is inserted *inside* the existing mapping,
    ///   at the mapping's own entry indent (per-level YAML indent is
    ///   config-dependent: a 4-space config nests at commands+4; the
    ///   default for an empty mapping is commands indent + 2);
    /// - otherwise (stage missing, empty, or without a `commands:` key)
    ///   a full `commands:` wrapper is injected after the stage anchor
    ///   or appended as a new stage section.
    ///
    /// Every marker sits alone on its own line and every emitted line
    /// ends with a newline — the END marker is never glued onto the next
    /// existing line (with comment-prefixed markers that would turn the
    /// following key into a YAML comment and silently delete it).
    /// Errors without modifying the file when: no config exists (never
    /// creates one, design D4); the stage key is not anchorable; the
    /// `commands:` key sits at an indent other than the children's
    /// (unrecognized structure); or exactly one block marker is present
    /// (unknown state, never guess).
    pub fn ensure_command_wired(
        root: &Path,
        stage: Stage,
        command: &str,
        run: &str,
        block: &BlockDef,
    ) -> Result<WiredOutcome, GitHooksError> {
        let path = config_path(root).ok_or_else(|| GitHooksError::MissingLefthookConfig {
            root: root.to_path_buf(),
        })?;
        let existing = std::fs::read_to_string(&path).map_err(|source| GitHooksError::Io {
            path: path.clone(),
            message: "failed to read lefthook config".to_string(),
            source,
        })?;

        // Idempotence at file level; exactly one marker = unknown state.
        if let Some(outcome) = marker_outcome(&path, &existing, block)? {
            return Ok(outcome);
        }

        match find_anchor(&existing, stage) {
            Err(()) => Err(GitHooksError::UnanchorableLefthookConfig {
                path,
                stage: stage.key().to_string(),
                message: "stage key is quoted or otherwise not anchorable at column 0".to_string(),
            }),
            Ok(Some(anchor)) => {
                wired_at_anchor(&path, &existing, anchor, stage, command, run, block)
            }
            Ok(None) => append_missing_stage(&path, &existing, stage, command, run, block),
        }
    }

    /// Wiring for a config that has a column-0 stage anchor: classify the
    /// stage section (empty / in-mapping / mismatched `commands:` indent /
    /// children without `commands:`) and inject accordingly.
    #[allow(clippy::too_many_arguments)]
    fn wired_at_anchor(
        path: &Path,
        existing: &str,
        anchor: usize,
        stage: Stage,
        command: &str,
        run: &str,
        block: &BlockDef,
    ) -> Result<WiredOutcome, GitHooksError> {
        let sec = section(existing, anchor);
        match (children_indent(sec), find_commands_key(sec)) {
            // Empty stage → wrapper directly after the anchor.
            (None, _) => insert_wrapper(path, existing, anchor, 2, command, run, block),
            // `commands:` at the children indent → in-mapping insert.
            (Some(_), Some((coff, ki))) if ki == children_indent(sec).unwrap() => {
                let commands_line_start = anchor + coff;
                let entry_indent = infer_entry_indent(existing, commands_line_start, ki);
                insert_after_line(
                    path,
                    existing,
                    commands_line_start,
                    &entry_at(entry_indent, command, run, block),
                )?;
                Ok(WiredOutcome::Injected)
            }
            // `commands:` at another indent — refuse to guess.
            (Some(_), Some((_, ki))) => Err(GitHooksError::UnanchorableLefthookConfig {
                path: path.to_path_buf(),
                stage: stage.key().to_string(),
                message: format!(
                    "commands key is at indent {ki}, which differs from the stage's children indentation — unrecognized structure"
                ),
            }),
            // Children but no `commands:` key → wrapper at the
            // stage's own children indent.
            (Some(ci), None) => insert_wrapper(path, existing, anchor, ci, command, run, block),
        }
    }

    /// Inject a full `commands:` wrapper at `indent` right after the
    /// stage anchor line.
    #[allow(clippy::too_many_arguments)]
    fn insert_wrapper(
        path: &Path,
        existing: &str,
        anchor: usize,
        indent: usize,
        command: &str,
        run: &str,
        block: &BlockDef,
    ) -> Result<WiredOutcome, GitHooksError> {
        insert_after_line(
            path,
            existing,
            anchor,
            &wrapper_at(indent, command, run, block),
        )?;
        Ok(WiredOutcome::Injected)
    }

    /// Marker-level idempotence check: `Some(AlreadyWired)` when both
    /// markers are present, `Err(UnbalancedLefthookMarkers)` when exactly
    /// one is (unknown state, never guess), `None` when the block is
    /// absent and wiring may proceed.
    fn marker_outcome(
        path: &Path,
        existing: &str,
        block: &BlockDef,
    ) -> Result<Option<WiredOutcome>, GitHooksError> {
        let has_start = existing.contains(&block.start_marker);
        let has_end = existing.contains(&block.end_marker);
        if has_start && has_end {
            return Ok(Some(WiredOutcome::AlreadyWired));
        }
        if has_start || has_end {
            return Err(GitHooksError::UnbalancedLefthookMarkers {
                path: path.to_path_buf(),
                start: block.start_marker.clone(),
                end: block.end_marker.clone(),
            });
        }
        Ok(None)
    }

    /// Render the marker-guarded command entry at `indent` — every line
    /// newline-terminated, every marker alone on its own line.
    fn entry_at(indent: usize, command: &str, run: &str, block: &BlockDef) -> String {
        let pad = " ".repeat(indent);
        format!(
            "{pad}{start}\n{pad}{command}:\n{pad}  run: {run}\n{pad}{end}\n",
            start = block.start_marker,
            end = block.end_marker
        )
    }

    /// Render a full `commands:` wrapper at `indent`, nesting the entry at
    /// `indent + 2` (the default mapping-entry indent for an empty mapping).
    fn wrapper_at(indent: usize, command: &str, run: &str, block: &BlockDef) -> String {
        format!(
            "{}commands:\n{}",
            " ".repeat(indent),
            entry_at(indent + 2, command, run, block)
        )
    }

    /// Infer the entry indent of an existing `commands:` mapping: the
    /// first non-blank, non-comment line deeper than the key's indent;
    /// commands indent + 2 when the mapping is empty (or closes before
    /// any entry).
    fn infer_entry_indent(
        text: &str,
        commands_line_start: usize,
        commands_key_indent: usize,
    ) -> usize {
        let commands_line_end = text[commands_line_start..]
            .find('\n')
            .map_or(text.len(), |nl| commands_line_start + nl + 1);
        let mut entry_indent = commands_key_indent + 2;
        for line in text[commands_line_end..].split_inclusive('\n') {
            let trimmed = line.trim_end();
            let indent = line.len() - line.trim_start().len();
            if !trimmed.is_empty() && !trimmed.trim_start().starts_with('#') {
                if indent > commands_key_indent {
                    entry_indent = indent;
                }
                break; // mapping closed (or first entry seen)
            }
            if !trimmed.is_empty() && indent <= commands_key_indent {
                break; // mapping closed before any entry
            }
        }
        entry_indent
    }

    /// Insert `insertion` after the line containing byte offset
    /// `line_start`, keeping every emitted line newline-terminated.
    fn insert_after_line(
        path: &Path,
        text: &str,
        line_start: usize,
        insertion: &str,
    ) -> Result<(), GitHooksError> {
        let line_end = text[line_start..]
            .find('\n')
            .map_or(text.len(), |nl| line_start + nl + 1);
        let mut updated = String::with_capacity(text.len() + insertion.len() + 1);
        updated.push_str(&text[..line_end]);
        if line_end == text.len() && !text.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(insertion);
        updated.push_str(&text[line_end..]);
        std::fs::write(path, updated).map_err(|source| GitHooksError::Io {
            path: path.to_path_buf(),
            message: "failed to write lefthook config".to_string(),
            source,
        })
    }

    /// Missing stage section → append `stage:` + a commands wrapper at
    /// EOF, ensuring the file ends newline-terminated before appending.
    fn append_missing_stage(
        path: &Path,
        existing: &str,
        stage: Stage,
        command: &str,
        run: &str,
        block: &BlockDef,
    ) -> Result<WiredOutcome, GitHooksError> {
        let mut updated = existing.to_owned();
        if !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(stage.key());
        updated.push_str(":\n");
        updated.push_str(&wrapper_at(2, command, run, block));
        std::fs::write(path, updated).map_err(|source| GitHooksError::Io {
            path: path.to_path_buf(),
            message: "failed to write lefthook config".to_string(),
            source,
        })?;
        Ok(WiredOutcome::Injected)
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
    install_with_env(root, hook_name, marker, script, &[])
}

/// Like [`install`], but passes extra environment variables to any spawned
/// `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
fn install_with_env(
    root: &Path,
    hook_name: HookName,
    marker: &str,
    script: &str,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    let path = hook_path_with_env(root, &hook_name, envs)?;
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
    uninstall_with_env(root, hook_name, marker, &[])
}

/// Like [`uninstall`], but passes extra environment variables to any
/// spawned `git config` process (test seam — see [`effective_hooks_dir_with_env`]).
fn uninstall_with_env(
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

#[cfg(test)]
#[path = "git_hooks/tests.rs"]
mod tests;
