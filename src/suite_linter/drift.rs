//! `ManagedBlockDrift` lint: deployed managed blocks match regenerated
//! generator content (provenance-hash fast path, full-text fallback).

use std::path::Path;

use crate::managed_block::content_sha8;

use crate::managed_block::BlockDef;

use super::{LintCheck, LintResult, Severity};

// ── ManagedBlockDrift check ───────────────────────────────────────

/// One managed block to check for drift: its definition, where it lives,
/// the regenerated expected content, and the command that fixes it.
pub struct DriftTarget {
    /// Block definition (name + markers).
    pub block: BlockDef,
    /// File path holding the block, relative to the repo root.
    pub path: std::path::PathBuf,
    /// Expected (footer-free) block content, as the generator would emit it.
    pub expected: String,
    /// Caller-supplied fix command — only the owning tool knows its sync
    /// command, so drift findings carry it verbatim.
    pub fix: String,
}

impl DriftTarget {
    /// Create a new drift target.
    pub fn new(
        block: BlockDef,
        path: impl Into<std::path::PathBuf>,
        expected: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            block,
            path: path.into(),
            expected: expected.into(),
            fix: fix.into(),
        }
    }
}

/// Drift check for managed blocks (`add-artifact-provenance` §3).
///
/// For each registered [`DriftTarget`], compares the block on disk against
/// the regenerated expected content — via the provenance footer's content
/// hash where a footer exists (fast path), and via full-text comparison for
/// older footer-less blocks. Drift is a Warning (not breakage); a registered
/// block missing from its file is Advisory.
pub struct ManagedBlockDrift {
    targets: Vec<DriftTarget>,
}

impl ManagedBlockDrift {
    /// Create a drift check for the given targets.
    pub fn new(targets: Vec<DriftTarget>) -> Self {
        Self { targets }
    }
}

/// Split the provenance footer off block-internal content.
///
/// Returns the footer-free body and, when the last line is a provenance
/// footer, its `sha=` value. Matches the inject format
/// `{content}\n{footer}\n` — the separating newline is dropped too, so the
/// body hashes byte-identically to the generator's content.
fn split_provenance_footer(inner: &str) -> (&str, Option<&str>) {
    let trimmed = inner.trim_end_matches('\n');
    let last_line = match trimmed.rfind('\n') {
        Some(idx) => &trimmed[idx + 1..],
        None if !trimmed.is_empty() => trimmed,
        None => return (inner, None),
    };
    match parse_footer_sha(last_line) {
        Some(sha) => {
            let cut = trimmed.len() - last_line.len();
            (&trimmed[..cut.saturating_sub(1)], Some(sha))
        }
        None => (inner, None),
    }
}

/// Extract `sha=<value>` from a provenance footer line, if it is one.
fn parse_footer_sha(line: &str) -> Option<&str> {
    let rest = line
        .strip_prefix("<!-- provenance: ")?
        .strip_suffix(" -->")?;
    let idx = rest.find("sha=")?;
    let sha = &rest[idx + 4..];
    Some(sha.split_whitespace().next().unwrap_or(sha))
}

impl LintCheck for ManagedBlockDrift {
    fn name(&self) -> &'static str {
        "genesis.managed_block_drift"
    }

    fn description(&self) -> &'static str {
        "managed blocks match their regenerated generator content"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let mut findings = Vec::new();
        for target in &self.targets {
            findings.extend(check_target(repo_root, target));
        }
        Ok(findings)
    }
}

/// Advisory when the target's file is missing or unreadable.
fn missing_file(target: &DriftTarget) -> LintResult {
    advisory(
        target,
        format!(
            "managed block '{}' file not found: {}",
            target.block.name,
            target.path.display()
        ),
    )
}

/// Advisory when the file exists but carries no block markers.
fn absent_block(target: &DriftTarget) -> LintResult {
    advisory(
        target,
        format!(
            "managed block '{}' absent from {}",
            target.block.name,
            target.path.display()
        ),
    )
}

/// Check one drift target: missing file, absent block, or drifted body.
pub(super) fn check_target(repo_root: &Path, target: &DriftTarget) -> Vec<LintResult> {
    let Ok(content) = std::fs::read_to_string(repo_root.join(&target.path)) else {
        return vec![missing_file(target)];
    };
    let Some(inner) = extract_block(&content, target) else {
        return vec![absent_block(target)];
    };
    if has_drifted(inner, target) {
        return vec![LintResult::with_fix(
            drift_message(target),
            Severity::Warning,
            &target.fix,
        )];
    }
    Vec::new()
}

/// The drift finding message for a target.
fn drift_message(target: &DriftTarget) -> String {
    format!(
        "managed block '{}' in {} has drifted from generator output",
        target.block.name,
        target.path.display()
    )
}

/// An advisory `LintResult` carrying the target's fix hint.
fn advisory(target: &DriftTarget, message: String) -> LintResult {
    LintResult::with_fix(message, Severity::Advisory, &target.fix)
}

/// The block body between start/end markers, or `None` when markers are
/// absent or inverted.
fn extract_block<'a>(content: &'a str, target: &DriftTarget) -> Option<&'a str> {
    content
        .find(&target.block.start_marker)
        .zip(content.find(&target.block.end_marker))
        .filter(|(start, end)| start < end)
        .map(|(start, end)| &content[start + target.block.start_marker.len()..end])
}

/// Fast path: footer hash over the footer-free body. Fallback: footer-less
/// (older) blocks — full-text compare.
fn has_drifted(inner: &str, target: &DriftTarget) -> bool {
    match split_provenance_footer(inner) {
        (body, Some(sha)) => sha != content_sha8(body),
        (body, None) => body.trim() != target.expected.trim(),
    }
}
