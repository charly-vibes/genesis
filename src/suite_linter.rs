//! Suite-wide lint orchestrator.
//!
//! The suite_linter is an **orchestrator, not a monolith**. Each tool defines
//! its own checks via the `LintCheck` trait; genesis just runs them.
//!
//! - `LintCheck` trait — tools implement this for each check
//! - `LintResult` — severity + message + optional fix command
//! - `LinterRegistry` — tools register checks, genesis runs them
//! - `ManagedBlockDrift` — provenance-footer drift check for managed blocks

use std::path::Path;

use crate::managed_block::{BlockDef, content_sha8};

// ── Types ─────────────────────────────────────────────────────────────

/// Severity of a lint result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational — no action required.
    Advisory,
    /// Something is likely wrong — should be addressed.
    Warning,
    /// Something is definitely wrong — must be fixed.
    Error,
}

impl Severity {
    /// Returns `true` if this severity is at least as severe as `other`.
    pub fn is_at_least(&self, other: Severity) -> bool {
        let rank = |s: Severity| -> u8 {
            match s {
                Severity::Advisory => 0,
                Severity::Warning => 1,
                Severity::Error => 2,
            }
        };
        rank(*self) >= rank(other)
    }
}

/// A single lint result.
#[derive(Debug, Clone)]
pub struct LintResult {
    /// Human-readable message describing the issue.
    pub message: String,
    /// How severe the issue is.
    pub severity: Severity,
    /// Optional command to fix the issue (e.g., `"wai init"`).
    pub fix: Option<String>,
}

impl LintResult {
    /// Create a new lint result.
    pub fn new(message: impl Into<String>, severity: Severity) -> Self {
        Self {
            message: message.into(),
            severity,
            fix: None,
        }
    }

    /// Create a new lint result with a fix command.
    pub fn with_fix(
        message: impl Into<String>,
        severity: Severity,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            severity,
            fix: Some(fix.into()),
        }
    }

    /// Format the result as a human-readable string.
    pub fn format(&self, check_name: &str) -> String {
        let sev = match self.severity {
            Severity::Advisory => "advisory",
            Severity::Warning => "warning",
            Severity::Error => "error",
        };
        if let Some(ref fix) = self.fix {
            format!("[{}] [{}] {} — fix: {}", sev, check_name, self.message, fix)
        } else {
            format!("[{}] [{}] {}", sev, check_name, self.message)
        }
    }
}

// ── LintCheck trait ───────────────────────────────────────────────────

/// A single lint check that a tool can register.
///
/// Each tool defines its own checks by implementing this trait.
pub trait LintCheck: Send + Sync {
    /// Unique name for this check (e.g., `"testaruda.schema"`).
    fn name(&self) -> &'static str;

    /// Human-readable description.
    fn description(&self) -> &'static str;

    /// Run the check against the given repo root.
    ///
    /// Returns a list of results (usually 0 or 1, but a check may report
    /// multiple findings).
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>>;
}

// ── LinterRegistry ────────────────────────────────────────────────────

/// Adoption probe for the suite-wide evals guidelines
/// (`add-evals-guidelines`, task 5.3): verifies that a consumer repo's
/// eval docs reference the genesis guideline pages (the report contract
/// and the tier-ladder how-tos). Advisory — an adoption probe, not a gate.
/// Register it in the consumer's linter registry at startup.
pub struct EvalsGuidelinesAdoption;

/// Marker strings that count as "references the guidelines" — any one hit
/// per eval doc satisfies the probe.
const EVALS_GUIDELINE_MARKERS: &[&str] = &[
    "eval-report",
    "evals-guidelines",
    "evals-ci",
    "why-weak-readers",
];

impl LintCheck for EvalsGuidelinesAdoption {
    fn name(&self) -> &'static str {
        "genesis.evals_guidelines"
    }

    fn description(&self) -> &'static str {
        "eval docs reference the suite-wide evals-guidelines pages"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let docs = repo_root.join("docs");
        if !docs.is_dir() {
            return Ok(vec![LintResult::new(
                "no eval docs found — evals-guidelines not yet adopted \
                 (battery not started)",
                Severity::Advisory,
            )]);
        }

        // Eval docs: any `evals*.md` under docs/, any depth.
        let mut eval_docs: Vec<std::path::PathBuf> = Vec::new();
        let mut stack = vec![docs.clone()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                } else if let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && name.starts_with("evals")
                    && name.ends_with(".md")
                {
                    eval_docs.push(path);
                }
            }
        }

        if eval_docs.is_empty() {
            return Ok(vec![LintResult::new(
                "no eval docs found — evals-guidelines not yet adopted \
                 (battery not started)",
                Severity::Advisory,
            )]);
        }

        let mut findings = Vec::new();
        for doc in eval_docs {
            let content = std::fs::read_to_string(&doc)
                .map_err(|e| format!("cannot read {}: {e}", doc.display()))?;
            let references = EVALS_GUIDELINE_MARKERS.iter().any(|m| content.contains(m));
            if !references {
                let rel = doc.strip_prefix(repo_root).unwrap_or(&doc).display();
                findings.push(LintResult::with_fix(
                    format!(
                        "{rel} does not reference the suite-wide evals-guidelines pages \
                         (report contract / tier ladder) — cross-tool results are not \
                         comparable without the shared conventions"
                    ),
                    Severity::Advisory,
                    "see genesis docs: how-to/evals.md, how-to/evals-ci.md, \
                     reference/eval-report.md",
                ));
            }
        }
        Ok(findings)
    }
}

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
            let content = match std::fs::read_to_string(repo_root.join(&target.path)) {
                Ok(c) => c,
                Err(_) => {
                    findings.push(LintResult::with_fix(
                        format!(
                            "managed block '{}' file not found: {}",
                            target.block.name,
                            target.path.display()
                        ),
                        Severity::Advisory,
                        &target.fix,
                    ));
                    continue;
                }
            };
            let inner = match content
                .find(&target.block.start_marker)
                .zip(content.find(&target.block.end_marker))
                .filter(|(start, end)| start < end)
            {
                Some((start, end)) => &content[start + target.block.start_marker.len()..end],
                None => {
                    findings.push(LintResult::with_fix(
                        format!(
                            "managed block '{}' absent from {}",
                            target.block.name,
                            target.path.display()
                        ),
                        Severity::Advisory,
                        &target.fix,
                    ));
                    continue;
                }
            };
            let (body, footer_sha) = split_provenance_footer(inner);
            let drifted = match footer_sha {
                // Fast path: footer hash over the footer-free body.
                Some(sha) => sha != content_sha8(body),
                // Fallback: footer-less (older) blocks — full-text compare.
                None => body.trim() != target.expected.trim(),
            };
            if drifted {
                findings.push(LintResult::with_fix(
                    format!(
                        "managed block '{}' in {} has drifted from generator output",
                        target.block.name,
                        target.path.display()
                    ),
                    Severity::Warning,
                    &target.fix,
                ));
            }
        }
        Ok(findings)
    }
}

// ── LinterRegistry ────────────────────────────────────────────────────

/// A registry of lint checks that tools register at startup.
///
/// Genesis provides the orchestration; tools provide the checks.
#[derive(Default)]
pub struct LinterRegistry {
    checks: Vec<Box<dyn LintCheck>>,
}

impl LinterRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a lint check.
    pub fn register(&mut self, check: Box<dyn LintCheck>) {
        self.checks.push(check);
    }

    /// Register multiple lint checks at once.
    pub fn register_all(&mut self, checks: Vec<Box<dyn LintCheck>>) {
        self.checks.extend(checks);
    }

    /// Number of registered checks.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Get the names of all registered checks.
    pub fn check_names(&self) -> Vec<&'static str> {
        self.checks.iter().map(|c| c.name()).collect()
    }

    /// Find a check by name.
    pub fn find(&self, name: &str) -> Option<&dyn LintCheck> {
        self.checks
            .iter()
            .find(|c| c.name() == name)
            .map(|c| c.as_ref())
    }

    /// Run all registered checks against the given repo root.
    ///
    /// Returns a list of `(check, results)` pairs. Checks that error are
    /// reported as a single error result.
    pub fn run_all(&self, repo_root: &Path) -> Vec<(&dyn LintCheck, Vec<LintResult>)> {
        self.checks
            .iter()
            .map(|check| {
                let results = match check.run(repo_root) {
                    Ok(r) => r,
                    Err(e) => vec![LintResult::new(
                        format!("check failed: {}", e),
                        Severity::Error,
                    )],
                };
                (check.as_ref(), results)
            })
            .collect()
    }

    /// Run a single check by name.
    ///
    /// Returns `None` if no check with that name is registered.
    pub fn run_named(&self, name: &str, repo_root: &Path) -> Option<Vec<LintResult>> {
        let check = self.checks.iter().find(|c| c.name() == name)?;
        Some(match check.run(repo_root) {
            Ok(r) => r,
            Err(e) => vec![LintResult::new(
                format!("check '{}' failed: {}", name, e),
                Severity::Error,
            )],
        })
    }

    /// Run checks with a minimum severity threshold.
    ///
    /// Only returns results with severity >= `min_severity`.
    /// Results are NOT re-wrapped with check name — use `format()` on the
    /// result with the check name for display.
    pub fn run_filtered(&self, repo_root: &Path, min_severity: Severity) -> Vec<LintResult> {
        self.run_all(repo_root)
            .into_iter()
            .flat_map(|(_check, results)| {
                results
                    .into_iter()
                    .filter(|r| r.severity.is_at_least(min_severity))
            })
            .collect()
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(test)]
#[path = "suite_linter/tests.rs"]
mod tests;
