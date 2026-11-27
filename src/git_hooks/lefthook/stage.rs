//! Stage-section splicing for [`super::ensure_wired`]: idempotent
//! injection into an existing stage section, or append of a new section
//! with its `commands:` mapping.

use std::path::Path;

use super::Stage;
use super::anchor::{children_indent, find_commands_key, line_end_after};
use super::inject::ensure_trailing_newline;
use super::{GitHooksError, WiredOutcome, write_config};

/// Error for a `commands:` key at an unexpected indent — shared by
/// `ensure_wired` and `ensure_command_wired` (never splice at the wrong
/// level, genesis-5sm).
pub(super) fn commands_indent_error(path: &Path, stage: Stage, key_indent: usize) -> GitHooksError {
    GitHooksError::UnanchorableLefthookConfig {
        path: path.to_path_buf(),
        stage: stage.key().to_string(),
        message: format!(
            "commands key is at indent {key_indent}, which differs from the stage's children indentation — unrecognized structure"
        ),
    }
}

/// Stage section exists at `offset` (its text is `sec`): inject
/// idempotently into it.
///
/// Idempotence is per stage section ("the stage's section already
/// contains the block's content"). `is_column_zero_key` treats comments
/// as non-keys, so a stage section spans an injected block exactly; the
/// former file-level `contains(block_text)` clause satisfied later
/// stages from the first stage's block, leaving dual-section configs —
/// the common real shape — with only the first stage wired.
pub(super) fn wire_into_stage(
    path: &Path,
    existing: &str,
    offset: usize,
    stage_section: &str,
    block_text: &str,
    content: &str,
    stage: Stage,
) -> Result<WiredOutcome, GitHooksError> {
    if stage_section.contains(content) {
        return Ok(WiredOutcome::AlreadyWired);
    }
    let updated = splice_into_stage(path, existing, offset, stage_section, block_text, stage)?;
    write_config(path, &updated)?;
    Ok(WiredOutcome::Injected)
}

/// Splice `block_text` into the stage section `sec`: after the existing
/// `commands:` line when present (its offset is relative to the stage
/// section and re-based onto the full config), otherwise after the
/// anchor line with a `commands:` prefix.
///
/// genesis-r99: lefthook only honors commands declared under the
/// stage's `commands:` mapping — nest the block inside `commands:`,
/// emitting the mapping after the stage anchor when the stage lacks
/// one. Caller content is inserted verbatim: it is written for the
/// `commands:` child level.
///
/// genesis-5sm: a `commands:` key at an indent other than the stage's
/// children indent is unrecognized structure — refuse with the same
/// [`GitHooksError::UnanchorableLefthookConfig`] as
/// `ensure_command_wired` instead of splicing at the wrong level.
fn splice_into_stage(
    path: &Path,
    existing: &str,
    offset: usize,
    sec: &str,
    block_text: &str,
    stage: Stage,
) -> Result<String, GitHooksError> {
    let (insert_at, prefix) = commands_insert_point(path, existing, offset, sec, stage)?;
    Ok(splice_with_prefix(existing, insert_at, prefix, block_text))
}

/// Where the block goes: right after the `commands:` key's line when the
/// key exists (with its indent validated), else at the section end with a
/// new `commands:` prefix.
fn commands_insert_point(
    path: &Path,
    existing: &str,
    offset: usize,
    sec: &str,
    stage: Stage,
) -> Result<(usize, &'static str), GitHooksError> {
    let anchor_line_end = line_end_after(existing, offset);
    Ok(match find_commands_key(sec) {
        Some((cmd_offset, key_indent)) => {
            let children = children_indent(sec).expect("a stage with a commands key has children");
            if key_indent != children {
                return Err(commands_indent_error(path, stage, key_indent));
            }
            (offset + line_end_after(sec, cmd_offset), "")
        }
        None => (anchor_line_end, "  commands:\n"),
    })
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
pub(super) fn append_stage_with_commands(
    path: &Path,
    existing: &str,
    stage: Stage,
    block_text: &str,
) -> Result<WiredOutcome, GitHooksError> {
    let mut updated = ensure_trailing_newline(existing);
    updated.push_str(stage.key());
    updated.push_str(":\n  commands:\n");
    updated.push_str(block_text);
    updated.push('\n');
    write_config(path, &updated)?;
    Ok(WiredOutcome::Injected)
}
