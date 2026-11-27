//! Managed-block injection into lefthook config sections.

use super::Stage;
use super::anchor::{children_indent, find_anchor, find_commands_key, line_end_after, section};
use super::{GitHooksError, WiredOutcome, unanchorable_error, write_config};
use crate::managed_block::BlockDef;
use std::path::Path;

/// Everything one wiring pass needs besides where to act: the config
/// path, its current text, and the command being wired.
pub(super) struct Wiring<'a> {
    pub(super) path: &'a Path,
    pub(super) existing: &'a str,
    pub(super) stage: Stage,
    pub(super) command: &'a str,
    pub(super) run: &'a str,
    pub(super) block: &'a BlockDef,
}

/// Shape of the stage section being wired, from its children indent
/// and its `commands:` key (offset within the section, key indent).
enum SectionShape {
    /// No child lines at all.
    Empty,
    /// `commands:` sits at the children indent — insert into the mapping.
    InMapping { start: usize, key_indent: usize },
    /// `commands:` at an unexpected indent — refuse to guess.
    Mismatched { key_indent: usize },
    /// Children but no `commands:` key — add a wrapper at children indent.
    ChildrenWithoutCommands { indent: usize },
}

/// Classify the stage section for [`wired_at_anchor`].
fn classify_section(sec: &str) -> SectionShape {
    let children = children_indent(sec);
    match (children, find_commands_key(sec)) {
        (None, _) => SectionShape::Empty,
        (Some(ci), Some((start, ki))) if ki == ci => SectionShape::InMapping {
            start,
            key_indent: ki,
        },
        (Some(_), Some((_, ki))) => SectionShape::Mismatched { key_indent: ki },
        (Some(ci), None) => SectionShape::ChildrenWithoutCommands { indent: ci },
    }
}

/// Wiring for a config that has a column-0 stage anchor: classify the
/// stage section (empty / in-mapping / mismatched `commands:` indent /
/// children without `commands:`) and inject accordingly.
fn wired_at_anchor(w: &Wiring, anchor: usize) -> Result<WiredOutcome, GitHooksError> {
    match classify_section(section(w.existing, anchor)) {
        // Empty stage → wrapper directly after the anchor.
        SectionShape::Empty => insert_wrapper(w, anchor, 2),
        // `commands:` at the children indent → in-mapping insert.
        SectionShape::InMapping { start, key_indent } => {
            insert_into_mapping(w, anchor + start, key_indent)
        }
        // `commands:` at another indent — refuse to guess.
        SectionShape::Mismatched { key_indent } => Err(commands_indent_error(w, key_indent)),
        SectionShape::ChildrenWithoutCommands { indent } => insert_wrapper(w, anchor, indent),
    }
}

/// Wire into the anchored stage section, or append a new stage
/// section when the stage key is absent.
pub(super) fn wire_or_append(w: &Wiring) -> Result<WiredOutcome, GitHooksError> {
    match find_anchor(w.existing, w.stage) {
        Err(()) => Err(unanchorable_error(w.path, w.stage)),
        Ok(Some(anchor)) => wired_at_anchor(w, anchor),
        Ok(None) => append_missing_stage(w),
    }
}

/// Inject a full `commands:` wrapper at `indent` right after the
/// stage anchor line.
fn insert_wrapper(w: &Wiring, anchor: usize, indent: usize) -> Result<WiredOutcome, GitHooksError> {
    insert_after_line(w.path, w.existing, anchor, &wrapper_at(indent, w))?;
    Ok(WiredOutcome::Injected)
}

/// Wire `command` into the config: file-level marker idempotence
/// first (`Some` → already wired, unbalanced refused), then
/// anchor-or-append.
pub(super) fn wire_command(w: &Wiring) -> Result<WiredOutcome, GitHooksError> {
    if let Some(outcome) = marker_outcome(w)? {
        return Ok(outcome);
    }
    wire_or_append(w)
}

/// Marker-level idempotence check: `Some(AlreadyWired)` when both
/// markers are present, `Err(UnbalancedLefthookMarkers)` when exactly
/// one is (unknown state, never guess), `None` when the block is
/// absent and wiring may proceed.
fn marker_outcome(w: &Wiring) -> Result<Option<WiredOutcome>, GitHooksError> {
    let has_start = w.existing.contains(&w.block.start_marker);
    let has_end = w.existing.contains(&w.block.end_marker);
    if has_start && has_end {
        return Ok(Some(WiredOutcome::AlreadyWired));
    }
    if has_start || has_end {
        return Err(GitHooksError::UnbalancedLefthookMarkers {
            path: w.path.to_path_buf(),
            start: w.block.start_marker.clone(),
            end: w.block.end_marker.clone(),
        });
    }
    Ok(None)
}

/// Render the marker-guarded command entry at `indent` — every line
/// newline-terminated, every marker alone on its own line.
fn entry_at(indent: usize, w: &Wiring) -> String {
    let pad = " ".repeat(indent);
    format!(
        "{pad}{}\n{pad}{}:\n{pad}  run: {}\n{pad}{}\n",
        w.block.start_marker, w.command, w.run, w.block.end_marker
    )
}

/// Render a full `commands:` wrapper at `indent`, nesting the entry at
/// `indent + 2` (the default mapping-entry indent for an empty mapping).
fn wrapper_at(indent: usize, w: &Wiring) -> String {
    format!(
        "{}commands:\n{}",
        " ".repeat(indent),
        entry_at(indent + 2, w)
    )
}

/// What the first deciding line after the `commands:` key says about
/// the mapping's entry indent.
enum EntryScan {
    /// Blank line or comment nested inside the mapping — keep looking.
    Skip,
    /// Mapping closed before any entry — use the default indent.
    Stop,
    /// First mapping entry — use this line's indent.
    Entry(usize),
}

/// Decide [`EntryScan`] for one line given the `commands:` key indent.
fn entry_scan(line: &str, key_indent: usize) -> EntryScan {
    let trimmed = line.trim_end();
    let indent = line.len() - line.trim_start().len();
    if trimmed.is_empty() {
        return EntryScan::Skip;
    }
    if trimmed.trim_start().starts_with('#') {
        // A comment at or above the key indent closes the mapping.
        if indent <= key_indent {
            return EntryScan::Stop;
        }
        return EntryScan::Skip;
    }
    if indent > key_indent {
        return EntryScan::Entry(indent);
    }
    EntryScan::Stop
}

/// Infer the entry indent of an existing `commands:` mapping: the
/// first non-blank, non-comment line deeper than the key's indent;
/// commands indent + 2 when the mapping is empty (or closes before
/// any entry).
fn infer_entry_indent(text: &str, commands_line_start: usize, commands_key_indent: usize) -> usize {
    let rest = &text[line_end_after(text, commands_line_start)..];
    for line in rest.split_inclusive('\n') {
        match entry_scan(line, commands_key_indent) {
            EntryScan::Skip => {}
            EntryScan::Stop => break,
            EntryScan::Entry(indent) => return indent,
        }
    }
    commands_key_indent + 2
}

/// Insert `insertion` after the line containing byte offset
/// `line_start`, keeping every emitted line newline-terminated.
fn insert_after_line(
    path: &Path,
    text: &str,
    line_start: usize,
    insertion: &str,
) -> Result<(), GitHooksError> {
    let spliced = splice_after_line(text, line_end_after(text, line_start), insertion);
    write_config(path, &spliced)?;
    Ok(())
}

/// `text` with `insertion` spliced in after byte offset `line_end`,
/// adding the newline the anchor line lacked when it is the last one.
fn splice_after_line(text: &str, line_end: usize, insertion: &str) -> String {
    let mut updated = String::with_capacity(text.len() + insertion.len() + 1);
    updated.push_str(&text[..line_end]);
    if line_end == text.len() && !text.ends_with('\n') {
        updated.push('\n');
    }
    updated.push_str(insertion);
    updated.push_str(&text[line_end..]);
    updated
}

/// `s` guaranteed newline-terminated (appends one when missing).
pub(super) fn ensure_trailing_newline(s: &str) -> String {
    let mut out = s.to_owned();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// Missing stage section → append `stage:` + a commands wrapper at
/// EOF, ensuring the file ends newline-terminated before appending.
fn append_missing_stage(w: &Wiring) -> Result<WiredOutcome, GitHooksError> {
    let mut updated = ensure_trailing_newline(w.existing);
    updated.push_str(w.stage.key());
    updated.push_str(":\n");
    updated.push_str(&wrapper_at(2, w));
    write_config(w.path, &updated)?;
    Ok(WiredOutcome::Injected)
}

/// Error for a `commands:` key at an unexpected indent (refuse to guess).
fn commands_indent_error(w: &Wiring, ki: usize) -> GitHooksError {
    GitHooksError::UnanchorableLefthookConfig {
        path: w.path.to_path_buf(),
        stage: w.stage.key().to_string(),
        message: format!(
            "commands key is at indent {ki}, which differs from the stage's children indentation — unrecognized structure"
        ),
    }
}

/// Insert the marker-guarded entry inside the existing `commands:` mapping,
/// at the mapping's own entry indent (per-level YAML indent is
/// config-dependent: a 4-space config nests at commands+4; the default for
/// an empty mapping is commands indent + 2).
fn insert_into_mapping(
    w: &Wiring,
    commands_line_start: usize,
    key_indent: usize,
) -> Result<WiredOutcome, GitHooksError> {
    let entry_indent = infer_entry_indent(w.existing, commands_line_start, key_indent);
    insert_after_line(
        w.path,
        w.existing,
        commands_line_start,
        &entry_at(entry_indent, w),
    )?;
    Ok(WiredOutcome::Injected)
}
