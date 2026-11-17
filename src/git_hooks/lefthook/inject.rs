//! Managed-block injection into lefthook config sections.

use super::Stage;
use super::anchor::{children_indent, find_commands_key, section};
use super::{GitHooksError, WiredOutcome};
use crate::managed_block::BlockDef;
use std::path::Path;

/// Wiring for a config that has a column-0 stage anchor: classify the
/// stage section (empty / in-mapping / mismatched `commands:` indent /
/// children without `commands:`) and inject accordingly.
pub(super) fn wired_at_anchor(
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
            insert_into_mapping(path, existing, anchor + coff, ki, command, run, block)
        }
        // `commands:` at another indent — refuse to guess.
        (Some(_), Some((_, ki))) => Err(commands_indent_error(path, stage, ki)),
        // Children but no `commands:` key → wrapper at the
        // stage's own children indent.
        (Some(ci), None) => insert_wrapper(path, existing, anchor, ci, command, run, block),
    }
}

/// Inject a full `commands:` wrapper at `indent` right after the
/// stage anchor line.
#[allow(clippy::too_many_arguments)]
pub(super) fn insert_wrapper(
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
pub(super) fn marker_outcome(
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
pub(super) fn infer_entry_indent(
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
pub(super) fn insert_after_line(
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
pub(super) fn append_missing_stage(
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

/// Error for a `commands:` key at an unexpected indent (refuse to guess).
fn commands_indent_error(path: &Path, stage: Stage, ki: usize) -> GitHooksError {
    GitHooksError::UnanchorableLefthookConfig {
        path: path.to_path_buf(),
        stage: stage.key().to_string(),
        message: format!(
            "commands key is at indent {ki}, which differs from the stage's children indentation — unrecognized structure"
        ),
    }
}

/// Insert the marker-guarded entry inside the existing `commands:` mapping,
/// at the mapping's own entry indent (per-level YAML indent is
/// config-dependent: a 4-space config nests at commands+4; the default for
/// an empty mapping is commands indent + 2).
#[allow(clippy::too_many_arguments)]
fn insert_into_mapping(
    path: &Path,
    existing: &str,
    commands_line_start: usize,
    ki: usize,
    command: &str,
    run: &str,
    block: &BlockDef,
) -> Result<WiredOutcome, GitHooksError> {
    let entry_indent = infer_entry_indent(existing, commands_line_start, ki);
    insert_after_line(
        path,
        existing,
        commands_line_start,
        &entry_at(entry_indent, command, run, block),
    )?;
    Ok(WiredOutcome::Injected)
}
