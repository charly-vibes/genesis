//! Block splicing helpers for [`super::BlockInjector`]: prepend, span
//! location, and in-place replacement of a managed block's text.

use std::path::Path;

use super::BlockDef;

/// Prepend `block_text` ahead of `existing` content, writing to `path`.
pub(super) fn prepend_block(path: &Path, existing: &str, block_text: &str) -> std::io::Result<()> {
    let new_content = format!("{block_text}\n\n{existing}");
    std::fs::write(path, new_content)
}

/// `NotFound` error for a block name missing from the registry.
pub(super) fn unknown_block_error(block_name: &str) -> std::io::Error {
    std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("block '{}' not registered", block_name),
    )
}

/// Byte span `[start, end_pos)` covered by `block`'s markers in `existing`.
///
/// The end marker is searched only *after* the start marker, mirroring
/// [`super::BlockInjector::read_block`]'s guard: a file that merely quotes the end
/// marker before the real block must not have the text between the quoted
/// and real markers spliced away.
pub(super) fn block_span(existing: &str, block: &BlockDef) -> Option<(usize, usize)> {
    let start = existing.find(&block.start_marker)?;
    let body = &existing[start + block.start_marker.len()..];
    let end_rel = body.find(&block.end_marker)?;
    Some((
        start,
        start + block.start_marker.len() + end_rel + block.end_marker.len(),
    ))
}

/// Replace `existing[start..end_pos]` with `block_text`.
pub(super) fn splice_block(
    existing: &str,
    start: usize,
    end_pos: usize,
    block_text: &str,
) -> String {
    let mut new_content = String::with_capacity(existing.len() + 512);
    new_content.push_str(&existing[..start]);
    new_content.push_str(block_text);
    new_content.push_str(&existing[end_pos..]);
    new_content
}
