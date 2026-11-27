//! Lefthook config wiring on top of [`crate::managed_block`] (design D4:
//! marker-tagged, idempotent blocks; no YAML parser, no bespoke string
//! surgery). Donor: espectacular `init.rs` (rebuilt), wai doctor (wiring
//! check semantics).

mod anchor;
mod inject;
mod stage;

use super::GitHooksError;
use crate::managed_block::BlockDef;
use anchor::{find_anchor, section};
use inject::{Wiring, wire_command};
use stage::{append_stage_with_commands, wire_into_stage};
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

/// Resolve the config path or fail — never creates a config (design D4).
fn config_path_or_error(root: &Path) -> Result<PathBuf, GitHooksError> {
    config_path(root).ok_or_else(|| GitHooksError::MissingLefthookConfig {
        root: root.to_path_buf(),
    })
}

pub fn ensure_wired(
    root: &Path,
    stage: Stage,
    block: &BlockDef,
    content: &str,
) -> Result<WiredOutcome, GitHooksError> {
    let path = config_path_or_error(root)?;
    validate_markers(&path, stage, block)?;
    let existing = read_config(&path)?;
    ensure_balanced_markers(&path, &existing, block)?;
    let block_text = format!("{}{}{}", block.start_marker, content, block.end_marker);
    wire_or_append(&path, &existing, stage, &block_text, content)
}

/// Route the block into the stage's section when an anchor exists,
/// or append a fresh stage section otherwise.
fn wire_or_append(
    path: &Path,
    existing: &str,
    stage: Stage,
    block_text: &str,
    content: &str,
) -> Result<WiredOutcome, GitHooksError> {
    match find_anchor(existing, stage) {
        Err(()) => Err(unanchorable_error(path, stage)),
        Ok(Some(offset)) => {
            let sec = section(existing, offset);
            wire_into_stage(path, existing, offset, sec, block_text, content, stage)
        }
        Ok(None) => append_stage_with_commands(path, existing, stage, block_text),
    }
}

/// Refuse markers that would be emitted as unparseable YAML
/// (genesis-5sm): a bare HTML-comment marker (`<!-- NAME:START -->`,
/// [`BlockDef::new`]'s default) lands in the config as a column-0
/// non-comment line and lefthook cannot parse the file afterwards —
/// markers must be comment-prefixed (`# <!-- NAME:START -->`).
fn validate_markers(path: &Path, stage: Stage, block: &BlockDef) -> Result<(), GitHooksError> {
    for marker in [&block.start_marker, &block.end_marker] {
        if marker.trim_start().starts_with("<!--") && !marker.starts_with('#') {
            return Err(GitHooksError::UnanchorableLefthookConfig {
                path: path.to_path_buf(),
                stage: stage.key().to_string(),
                message: format!(
                    "marker {marker:?} is a bare HTML comment: emitted into the config it produces unparseable YAML — declare comment-prefixed markers (e.g. \"# <!-- {name}:START -->\")",
                    name = block.name
                ),
            });
        }
    }
    Ok(())
}

/// genesis-5sm: marker balance is checked file-wide by COUNT, not
/// presence — per-stage blocks legitimately appear once per stage
/// section, so equal non-zero counts are healthy; differing counts are
/// an unknown state and wiring refuses unmodified (same discipline as
/// `ensure_command_wired`'s `marker_outcome`).
fn ensure_balanced_markers(
    path: &Path,
    existing: &str,
    block: &BlockDef,
) -> Result<(), GitHooksError> {
    let starts = existing.matches(&block.start_marker).count();
    let ends = existing.matches(&block.end_marker).count();
    if starts != ends {
        return Err(GitHooksError::UnbalancedLefthookMarkers {
            path: path.to_path_buf(),
            start: block.start_marker.clone(),
            end: block.end_marker.clone(),
        });
    }
    Ok(())
}

/// Read a lefthook config file into a string.
fn read_config(path: &Path) -> Result<String, GitHooksError> {
    std::fs::read_to_string(path).map_err(|source| GitHooksError::Io {
        path: path.to_path_buf(),
        message: "failed to read lefthook config".to_string(),
        source,
    })
}

/// Build the "stage key is not anchorable" error.
fn unanchorable_error(path: &Path, stage: Stage) -> GitHooksError {
    GitHooksError::UnanchorableLefthookConfig {
        path: path.to_path_buf(),
        stage: stage.key().to_string(),
        message: "stage key is quoted or otherwise not anchorable at column 0".to_string(),
    }
}

/// Write a lefthook config file.
fn write_config(path: &Path, updated: &str) -> Result<(), GitHooksError> {
    std::fs::write(path, updated).map_err(|source| GitHooksError::Io {
        path: path.to_path_buf(),
        message: "failed to write lefthook config".to_string(),
        source,
    })
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
    let path = config_path_or_error(root)?;
    validate_markers(&path, stage, block)?;
    let existing = read_config(&path)?;
    let wiring = Wiring {
        path: &path,
        existing: &existing,
        stage,
        command,
        run,
        block,
    };
    wire_command(&wiring)
}

/// Report whether `command` appears in a stage's section of the
/// lefthook config, for doctor-style wiring checks (design D5:
/// stage-scoped scan — a pre-push command must not satisfy a
/// pre-commit check). Returns `false` when the command is absent, the
/// stage section is missing, or no config exists.
///
/// Degradation (explicit, genesis-5sm): when the config exists but
/// cannot be read (permissions, invalid UTF-8), this also returns
/// `false` — indistinguishable from "not wired". Doctor-style callers
/// that must tell the two apart should re-read the config directly
/// instead of expanding this API with a fallible variant.
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
