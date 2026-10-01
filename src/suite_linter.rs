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
mod tests {
    use super::*;
    use std::path::PathBuf;

    // ── Helpers ───────────────────────────────────────────────────────

    /// A mock check that always passes.
    struct PassingCheck;

    impl LintCheck for PassingCheck {
        fn name(&self) -> &'static str {
            "passing"
        }
        fn description(&self) -> &'static str {
            "Always passes"
        }
        fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![])
        }
    }

    /// A mock check that always reports a warning.
    struct WarningCheck;

    impl LintCheck for WarningCheck {
        fn name(&self) -> &'static str {
            "warning"
        }
        fn description(&self) -> &'static str {
            "Always warns"
        }
        fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::new(
                "something looks suspicious",
                Severity::Warning,
            )])
        }
    }

    /// A mock check that always fails with an error.
    struct ErrorCheck;

    impl LintCheck for ErrorCheck {
        fn name(&self) -> &'static str {
            "error"
        }
        fn description(&self) -> &'static str {
            "Always errors"
        }
        fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::new(
                "config file is missing",
                Severity::Error,
            )])
        }
    }

    /// A mock check that panics internally.
    struct PanicCheck;

    impl LintCheck for PanicCheck {
        fn name(&self) -> &'static str {
            "panic"
        }
        fn description(&self) -> &'static str {
            "Always panics"
        }
        fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Err("internal failure".into())
        }
    }

    /// A mock check with a fix command.
    #[allow(dead_code)]
    struct FixableCheck;

    impl LintCheck for FixableCheck {
        fn name(&self) -> &'static str {
            "fixable"
        }
        fn description(&self) -> &'static str {
            "Has a fix command"
        }
        fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::with_fix(
                "missing config",
                Severity::Error,
                "tool init",
            )])
        }
    }

    fn test_root() -> PathBuf {
        std::env::temp_dir().join("genesis-suite-linter-test")
    }

    // ── Severity ──────────────────────────────────────────────────────

    #[test]
    fn test_severity_advisory_is_least_severe() {
        assert!(Severity::Advisory.is_at_least(Severity::Advisory));
        assert!(!Severity::Advisory.is_at_least(Severity::Warning));
        assert!(!Severity::Advisory.is_at_least(Severity::Error));
    }

    #[test]
    fn test_severity_warning_is_middle() {
        assert!(Severity::Warning.is_at_least(Severity::Advisory));
        assert!(Severity::Warning.is_at_least(Severity::Warning));
        assert!(!Severity::Warning.is_at_least(Severity::Error));
    }

    #[test]
    fn test_severity_error_is_most_severe() {
        assert!(Severity::Error.is_at_least(Severity::Advisory));
        assert!(Severity::Error.is_at_least(Severity::Warning));
        assert!(Severity::Error.is_at_least(Severity::Error));
    }

    // ── LintResult ────────────────────────────────────────────────────

    #[test]
    fn test_lint_result_new() {
        let r = LintResult::new("something wrong", Severity::Warning);
        assert_eq!(r.message, "something wrong");
        assert_eq!(r.severity, Severity::Warning);
        assert!(r.fix.is_none());
    }

    #[test]
    fn test_lint_result_with_fix() {
        let r = LintResult::with_fix("missing config", Severity::Error, "tool init");
        assert_eq!(r.fix, Some("tool init".to_string()));
    }

    #[test]
    fn test_lint_result_format_no_fix() {
        let r = LintResult::new("something wrong", Severity::Warning);
        let formatted = r.format("test.check");
        assert!(formatted.contains("[warning]"));
        assert!(formatted.contains("[test.check]"));
        assert!(formatted.contains("something wrong"));
        assert!(!formatted.contains("fix:"));
    }

    #[test]
    fn test_lint_result_format_with_fix() {
        let r = LintResult::with_fix("missing config", Severity::Error, "tool init");
        let formatted = r.format("test.check");
        assert!(formatted.contains("[error]"));
        assert!(formatted.contains("fix:"));
        assert!(formatted.contains("tool init"));
    }

    // ── LintCheck ─────────────────────────────────────────────────────

    #[test]
    fn test_passing_check_returns_no_results() {
        let check = PassingCheck;
        let results = check.run(&test_root()).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_warning_check_returns_warning() {
        let check = WarningCheck;
        let results = check.run(&test_root()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn test_error_check_returns_error() {
        let check = ErrorCheck;
        let results = check.run(&test_root()).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
    }

    // ── LinterRegistry ────────────────────────────────────────────────

    #[test]
    fn test_registry_empty_by_default() {
        let reg = LinterRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
    }

    #[test]
    fn test_registry_register_one() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_registry_register_all() {
        let mut reg = LinterRegistry::new();
        reg.register_all(vec![
            Box::new(PassingCheck),
            Box::new(WarningCheck),
            Box::new(ErrorCheck),
        ]);
        assert_eq!(reg.len(), 3);
    }

    #[test]
    fn test_registry_check_names() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        reg.register(Box::new(WarningCheck));
        let names = reg.check_names();
        assert!(names.contains(&"passing"));
        assert!(names.contains(&"warning"));
    }

    #[test]
    fn test_registry_find() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        let found = reg.find("passing");
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "passing");
    }

    #[test]
    fn test_registry_find_unknown() {
        let reg = LinterRegistry::new();
        assert!(reg.find("nonexistent").is_none());
    }

    #[test]
    fn test_registry_run_all() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        reg.register(Box::new(WarningCheck));
        reg.register(Box::new(ErrorCheck));

        let results = reg.run_all(&test_root());
        assert_eq!(results.len(), 3);

        // Passing check should have 0 results
        let (passing_check, passing_results) = &results[0];
        assert_eq!(passing_check.name(), "passing");
        assert!(passing_results.is_empty());

        // Warning check should have 1 warning
        let (warning_check, warning_results) = &results[1];
        assert_eq!(warning_check.name(), "warning");
        assert_eq!(warning_results[0].severity, Severity::Warning);

        // Error check should have 1 error
        let (error_check, error_results) = &results[2];
        assert_eq!(error_check.name(), "error");
        assert_eq!(error_results[0].severity, Severity::Error);
    }

    #[test]
    fn test_registry_run_named() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        reg.register(Box::new(ErrorCheck));

        let results = reg.run_named("error", &test_root());
        assert!(results.is_some());
        assert_eq!(results.unwrap().len(), 1);
    }

    #[test]
    fn test_registry_run_named_unknown() {
        let reg = LinterRegistry::new();
        assert!(reg.run_named("nonexistent", &test_root()).is_none());
    }

    #[test]
    fn test_registry_run_named_panicking_check() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PanicCheck));

        let results = reg.run_named("panic", &test_root());
        assert!(results.is_some());
        let results = results.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
        assert!(
            results[0].message.contains("failed"),
            "expected message to contain 'failed', got: {}",
            results[0].message
        );
    }

    #[test]
    fn test_registry_run_filtered_includes_errors_and_warnings() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(WarningCheck));
        reg.register(Box::new(ErrorCheck));

        let results = reg.run_filtered(&test_root(), Severity::Warning);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_registry_run_filtered_excludes_advisory() {
        // Create an advisory check
        struct AdvisoryCheck;
        impl LintCheck for AdvisoryCheck {
            fn name(&self) -> &'static str {
                "advisory"
            }
            fn description(&self) -> &'static str {
                "Always advisory"
            }
            fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
                Ok(vec![LintResult::new("info", Severity::Advisory)])
            }
        }

        let mut reg = LinterRegistry::new();
        reg.register(Box::new(AdvisoryCheck));
        reg.register(Box::new(ErrorCheck));

        let results = reg.run_filtered(&test_root(), Severity::Warning);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
    }

    #[test]
    fn test_registry_run_filtered_only_errors() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(WarningCheck));
        reg.register(Box::new(ErrorCheck));

        let results = reg.run_filtered(&test_root(), Severity::Error);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Error);
    }

    #[test]
    fn test_registry_run_all_handles_panicking_check() {
        let mut reg = LinterRegistry::new();
        reg.register(Box::new(PassingCheck));
        reg.register(Box::new(PanicCheck));

        let results = reg.run_all(&test_root());
        assert_eq!(results.len(), 2);

        // The panicking check should be caught and reported as an error
        let (name, check_results) = &results[1];
        assert_eq!(name.name(), "panic");
        assert_eq!(check_results[0].severity, Severity::Error);
        assert!(check_results[0].message.contains("check failed"));
    }

    // ── LintResult formatting ─────────────────────────────────────────

    #[test]
    fn test_format_with_fix_includes_command() {
        let r = LintResult::with_fix("missing config", Severity::Error, "tool init");
        let formatted = r.format("test.check");
        assert_eq!(
            formatted,
            "[error] [test.check] missing config — fix: tool init"
        );
    }

    #[test]
    fn test_format_without_fix_omits_command() {
        let r = LintResult::new("all good", Severity::Advisory);
        let formatted = r.format("test.check");
        assert_eq!(formatted, "[advisory] [test.check] all good");
    }

    // ── EvalsGuidelinesAdoption (add-evals-guidelines) ────────────────

    fn write_repo(files: &[(&str, &str)]) -> PathBuf {
        let dir = tempfile::tempdir().expect("tempdir");
        for (path, content) in files {
            let full = dir.path().join(path);
            std::fs::create_dir_all(full.parent().unwrap()).unwrap();
            std::fs::write(full, content).unwrap();
        }
        // Leak the temp dir so it outlives the test body.
        let path = dir.path().to_path_buf();
        std::mem::forget(dir);
        path
    }

    #[test]
    fn test_evals_adoption_passes_when_docs_reference_guidelines() {
        let root = write_repo(&[(
            "docs/how-to/evals.md",
            "# Evals\n\nSee the [report contract](../reference/eval-report.md).\n",
        )]);
        let results = EvalsGuidelinesAdoption.run(&root).unwrap();
        assert!(results.is_empty(), "expected no findings, got {results:?}");
    }

    #[test]
    fn test_evals_adoption_flags_docs_without_reference() {
        let root = write_repo(&[(
            "docs/how-to/evals.md",
            "# Evals\n\nMy battery runs on pushes.\n",
        )]);
        let results = EvalsGuidelinesAdoption.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Advisory);
        assert!(results[0].message.contains("docs/how-to/evals.md"));
        assert!(results[0].fix.is_some());
    }

    #[test]
    fn test_evals_adoption_advisory_when_no_eval_docs() {
        let root = write_repo(&[("README.md", "# my-tool\n")]);
        let results = EvalsGuidelinesAdoption.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Advisory);
        assert!(results[0].message.contains("no eval docs"));
    }

    #[test]
    fn test_evals_adoption_ignores_non_eval_docs() {
        let root = write_repo(&[("docs/how-to/fixture.md", "# Fixture\n")]);
        let results = EvalsGuidelinesAdoption.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].message.contains("no eval docs"));
    }

    // ── ManagedBlockDrift ─────────────────────────────────────────────

    use crate::managed_block::{BlockInjector, BlockRegistry};

    fn drift_target(rel: &str, expected: &str, fix: &str) -> DriftTarget {
        DriftTarget::new(
            BlockDef::new("WAI"),
            PathBuf::from(rel),
            expected.to_string(),
            fix.to_string(),
        )
    }

    /// A file whose block carries a footer with a stale (wrong) sha.
    fn drifted_footer_file(expected: &str) -> String {
        format!(
            "# head\n\n{}stale hand-edited body\n<!-- provenance: generator=genesis version=0.1.0 source=WAI sha={} -->\n{}",
            BlockDef::new("WAI").start_marker,
            content_sha8(expected),
            BlockDef::new("WAI").end_marker,
        )
    }

    #[test]
    fn test_drift_reported_with_file_block_and_fix() {
        let expected = "# generated\ncontent";
        let root = write_repo(&[("AGENTS.md", &drifted_footer_file(expected))]);
        let check =
            ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
        let results = check.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
        assert!(
            results[0].message.contains("AGENTS.md"),
            "names file: {}",
            results[0].message
        );
        assert!(
            results[0].message.contains("WAI"),
            "names block: {}",
            results[0].message
        );
        assert_eq!(results[0].fix.as_deref(), Some("my-tool init"));
    }

    #[test]
    fn test_current_block_no_finding() {
        let expected = "# generated\ncontent";
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        let injector = BlockInjector::new(reg).with_provenance("genesis");
        let root = write_repo(&[("AGENTS.md", "# head\n")]);
        injector
            .inject(&root.join("AGENTS.md"), "WAI", expected)
            .unwrap();
        let check =
            ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
        assert!(check.run(&root).unwrap().is_empty());
    }

    #[test]
    fn test_footerless_drift_full_text_fallback() {
        // Older blocks have no footer — drift is detected by full-text compare.
        let block = BlockDef::new("WAI");
        let file = format!("{}old body{}", block.start_marker, block.end_marker);
        let root = write_repo(&[("AGENTS.md", &file)]);
        let check =
            ManagedBlockDrift::new(vec![drift_target("AGENTS.md", "new body", "my-tool init")]);
        let results = check.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);

        // Matching content (no footer) is clean.
        let file = format!("{}new body{}", block.start_marker, block.end_marker);
        let root = write_repo(&[("AGENTS.md", &file)]);
        assert!(check.run(&root).unwrap().is_empty());
    }

    #[test]
    fn test_hand_edited_block_is_finding_not_crash() {
        let expected = "# generated\ncontent";
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        let injector = BlockInjector::new(reg).with_provenance("genesis");
        let root = write_repo(&[("AGENTS.md", "# head\n")]);
        injector
            .inject(&root.join("AGENTS.md"), "WAI", expected)
            .unwrap();
        // Hand-edit: markers intact, body changed, footer untouched.
        let edited = std::fs::read_to_string(root.join("AGENTS.md"))
            .unwrap()
            .replace("content", "hand-edited");
        std::fs::write(root.join("AGENTS.md"), edited).unwrap();
        let check =
            ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
        let results = check.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
    }

    #[test]
    fn test_missing_block_is_advisory() {
        let root = write_repo(&[("AGENTS.md", "# head — no block here\n")]);
        let check =
            ManagedBlockDrift::new(vec![drift_target("AGENTS.md", "anything", "my-tool init")]);
        let results = check.run(&root).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Advisory);
    }

    #[test]
    fn test_managed_block_drift_registry_wiring() {
        let expected = "# generated";
        let root = write_repo(&[("AGENTS.md", &drifted_footer_file(expected))]);
        let mut registry = LinterRegistry::new();
        registry.register(Box::new(ManagedBlockDrift::new(vec![drift_target(
            "AGENTS.md",
            expected,
            "my-tool init",
        )])));
        let results = registry
            .run_named("genesis.managed_block_drift", &root)
            .expect("check registered");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warning);
    }
}
