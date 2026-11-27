use std::path::{Component, Path, PathBuf};

use super::GitHooksError;
use super::hooks_dir::resolve_hooks_dir_with_env;

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
pub(super) fn hook_path_with_env(
    root: &Path,
    hook_name: &HookName,
    envs: &[(String, String)],
) -> Result<PathBuf, GitHooksError> {
    if let HookName::Other(name) = hook_name {
        // genesis-5sm: refuse names that escape the hooks directory
        // before joining (`../crontab`, `/etc/passwd`).
        if Path::new(name)
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::RootDir))
        {
            return Err(GitHooksError::InvalidHookName { name: name.clone() });
        }
    }
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
pub(crate) fn owner_with_env(
    root: &Path,
    hook_name: &HookName,
    envs: &[(String, String)],
) -> Option<Owner> {
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
pub(crate) fn framework_with_env(root: &Path, envs: &[(String, String)]) -> Framework {
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
