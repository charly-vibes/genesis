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

        let eval_docs = collect_eval_docs(&docs)?;

        if eval_docs.is_empty() {
            return Ok(vec![LintResult::new(
                "no eval docs found — evals-guidelines not yet adopted \
                 (battery not started)",
                Severity::Advisory,
            )]);
        }

        let mut findings = Vec::new();
        for doc in eval_docs {
            if let Some(finding) = check_eval_doc(repo_root, &doc) {
                findings.push(finding);
            }
        }
        Ok(findings)
    }
}

/// Advisory finding when `doc` does not reference any guideline marker.
fn check_eval_doc(repo_root: &Path, doc: &std::path::Path) -> Option<LintResult> {
    let content = std::fs::read_to_string(doc)
        .map_err(|e| format!("cannot read {}: {e}", doc.display()))
        .ok()?;
    let references = EVALS_GUIDELINE_MARKERS.iter().any(|m| content.contains(m));
    if references {
        return None;
    }
    let rel = doc.strip_prefix(repo_root).unwrap_or(doc).display();
    Some(LintResult::with_fix(
        format!(
            "{rel} does not reference the suite-wide evals-guidelines pages \
             (report contract / tier ladder) — cross-tool results are not \
             comparable without the shared conventions"
        ),
        Severity::Advisory,
        "see genesis docs: how-to/evals.md, how-to/evals-ci.md, \
         reference/eval-report.md",
    ))
}

mod drift;

pub use drift::{DriftTarget, ManagedBlockDrift};

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
#[path = "suite_linter/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "suite_linter/tests_drift.rs"]
mod tests_drift;

/// Collect `evals*.md` files under `docs/`, any depth.
fn collect_eval_docs(
    docs: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut eval_docs: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![docs.to_path_buf()];
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
    Ok(eval_docs)
}
