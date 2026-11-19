//! Lefthook YAML section/anchor discovery (indentation heuristics, no
//! YAML parser — design D4).

use super::Stage;

fn is_column_zero_key(line: &str) -> bool {
    let first = line.chars().next();
    matches!(first, Some(c) if c != ' ' && c != '\t' && c != '#' && c != '\n')
        && !line.starts_with("<!--")
}

/// Indent of the stage section's first child line (skips the anchor
/// line itself, blank lines and comments). `None` when the stage is
/// empty — used to classify the wiring case.
pub(super) fn children_indent(section: &str) -> Option<usize> {
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
pub(super) fn find_commands_key(section: &str) -> Option<(usize, usize)> {
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
pub(super) fn find_anchor(content: &str, stage: Stage) -> Result<Option<usize>, ()> {
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
pub(super) fn section(content: &str, anchor_offset: usize) -> &str {
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
