//! Lefthook config wiring on top of [`crate::managed_block`] (design D4:
//! marker-tagged, idempotent blocks; no YAML parser, no bespoke string
//! surgery). Donor: espectacular `init.rs` (rebuilt), wai doctor (wiring
//! check semantics).

mod anchor;
mod inject;

use super::GitHooksError;
use crate::managed_block::BlockDef;
use anchor::{find_anchor, section};
use inject::{append_missing_stage, marker_outcome, wired_at_anchor};
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

pub fn ensure_wired(
    root: &Path,
    stage: Stage,
    block: &BlockDef,
    content: &str,
) -> Result<WiredOutcome, GitHooksError> {
    let path = config_path(root).ok_or_else(|| GitHooksError::MissingLefthookConfig {
        root: root.to_path_buf(),
    })?;
    let existing = read_config(&path)?;
    let block_text = format!("{}{}{}", block.start_marker, content, block.end_marker);

    match find_anchor(&existing, stage) {
        Err(()) => Err(unanchorable_error(&path, stage)),
        Ok(Some(offset)) => wire_into_stage(&path, &existing, offset, stage, &block_text, content),
        Ok(None) => append_stage_with_commands(&path, &existing, stage, &block_text),
    }
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

/// Stage section exists at `offset`: inject idempotently into it.
///
/// Idempotence is per stage section ("the stage's section already
/// contains the block's content"). `is_column_zero_key` treats comments
/// as non-keys, so a stage section spans an injected block exactly; the
/// former file-level `contains(block_text)` clause satisfied later
/// stages from the first stage's block, leaving dual-section configs —
/// the common real shape — with only the first stage wired.
fn wire_into_stage(
    path: &Path,
    existing: &str,
    offset: usize,
    _stage: Stage,
    block_text: &str,
    content: &str,
) -> Result<WiredOutcome, GitHooksError> {
    if section(existing, offset).contains(content) {
        return Ok(WiredOutcome::AlreadyWired);
    }
    // genesis-r99: lefthook only honors commands declared under the
    // stage's `commands:` mapping — nest the block inside `commands:`,
    // emitting the mapping after the stage anchor when the stage lacks
    // one. Caller content is inserted verbatim: it is written for the
    // `commands:` child level.
    let stage_section = section(existing, offset);
    let anchor_line_end = existing[offset..]
        .find('\n')
        .map_or(existing.len(), |nl| offset + nl + 1);
    let (insert_at, prefix) = match commands_key_end(stage_section) {
        // `commands_end` is relative to the stage section: re-base it
        // onto the full config before splicing.
        Some(end) => (offset + end, ""),
        None => (anchor_line_end, "  commands:\n"),
    };
    let updated = splice_with_prefix(existing, insert_at, prefix, block_text);
    write_config(path, &updated)?;
    Ok(WiredOutcome::Injected)
}

/// Byte offset just past the stage section's `commands:` line, if any.
fn commands_key_end(stage_section: &str) -> Option<usize> {
    let mut scanned = 0usize;
    for (i, line) in stage_section.split_inclusive('\n').enumerate() {
        if i > 0 {
            let trimmed = line.trim_end();
            if trimmed.trim() == "commands:" && !trimmed.starts_with('#') {
                return Some(scanned + line.len());
            }
        }
        scanned += line.len();
    }
    None
}

/// Splice `block_text` (plus `prefix`) into `existing` at `insert_at`.
///
/// The block always ends on its own line: without this, caller content
/// ending in `\n` leaves the END marker glued onto the next existing
/// line — with comment-prefixed markers that line becomes a YAML
/// comment and the following key is silently deleted (genesis-au8).
fn splice_with_prefix(existing: &str, insert_at: usize, prefix: &str, block_text: &str) -> String {
    let mut updated = String::with_capacity(existing.len() + block_text.len() + prefix.len() + 2);
    updated.push_str(&existing[..insert_at]);
    if insert_at == existing.len() {
        updated.push('\n'); // anchor/commands was the last line without a newline
    }
    updated.push_str(prefix);
    updated.push_str(block_text);
    if !block_text.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(&existing[insert_at..]);
    updated
}

/// genesis-r99: appended stages also nest the block inside `commands:`
/// so the wired command is a real lefthook command, not a
/// silently-ignored stage-level key.
fn append_stage_with_commands(
    path: &Path,
    existing: &str,
    stage: Stage,
    block_text: &str,
) -> Result<WiredOutcome, GitHooksError> {
    let mut updated = existing.to_string();
    if !updated.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(stage.key());
    updated.push_str(":\n  commands:\n");
    updated.push_str(block_text);
    updated.push('\n');
    write_config(path, &updated)?;
    Ok(WiredOutcome::Injected)
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
        Err(()) => Err(unanchorable_error(&path, stage)),
        Ok(Some(anchor)) => wired_at_anchor(&path, &existing, anchor, stage, command, run, block),
        Ok(None) => append_missing_stage(&path, &existing, stage, command, run, block),
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
