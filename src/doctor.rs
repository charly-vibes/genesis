//! Doctor framework — structured diagnostics with auto-fix support.
//!
//! Extends the [`suite_linter`] pattern with a full doctor CLI workflow:
//! checks with auto-fix, structured reports, JSON envelope serialization,
//! and exit-code handling.
//!
//! ## Design
//!
//! - [`DoctorCheck`] trait — checks that can optionally auto-fix
//! - [`DoctorRunner`] — runs a collection of checks, handles `--fix`
//! - [`DoctorReport`] — structured result with summary, serializable
//! - Uses [`LintResult`] and [`Severity`] from [`suite_linter`]
//!
//! ## Usage
//!
//! ```rust
//! use genesis::doctor::{DoctorCheck, DoctorRunner, DoctorReport};
//! use genesis::suite_linter::{LintResult, Severity};
//! use std::path::Path;
//!
//! struct MyCheck;
//! impl DoctorCheck for MyCheck {
//!     fn name(&self) -> &'static str { "my-check" }
//!     fn description(&self) -> &'static str { "Checks something" }
//!     fn run(&self, _repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
//!         Ok(vec![]) // pass
//!     }
//! }
//!
//! let runner = DoctorRunner::new(vec![Box::new(MyCheck)]);
//! let report = runner.run(&std::env::current_dir().unwrap(), false).unwrap();
//! println!("pass={} warn={} fail={}", report.summary.pass, report.summary.warn, report.summary.fail);
//! ```

use crate::envelope::{Envelope, EnvelopeKind};
use crate::suite_linter::{LintResult, Severity};
use serde::Serialize;
use std::path::Path;

// ── CheckStatus ───────────────────────────────────────────────────────

/// Outcome status for a single doctor check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed — no issues found.
    Pass,
    /// Check found a non-blocking concern.
    Warn,
    /// Check found a blocking issue.
    Fail,
}

impl CheckStatus {
    /// Returns `true` if this is a passing status.
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckStatus::Pass)
    }

    /// Returns `true` if this is a warning or failure.
    pub fn is_issue(&self) -> bool {
        matches!(self, CheckStatus::Warn | CheckStatus::Fail)
    }

    /// Returns `true` if this is a failure (blocking).
    pub fn is_fail(&self) -> bool {
        matches!(self, CheckStatus::Fail)
    }
}

// ── CheckEntry ────────────────────────────────────────────────────────

/// A single check's result in the doctor report.
#[derive(Debug, Clone, Serialize)]
pub struct CheckEntry {
    /// Unique check name (e.g., `"wai.directory-structure"`).
    pub name: String,
    /// Human-readable description of what this check validates.
    pub description: String,
    /// The outcome status.
    pub status: CheckStatus,
    /// Human-readable message describing the result.
    pub message: String,
    /// Optional CLI command to fix the issue (e.g., `"wai init"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

impl CheckEntry {
    /// Create a passing check entry.
    pub fn pass(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            fix: None,
        }
    }

    /// Create a warning check entry.
    pub fn warn(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            fix: None,
        }
    }

    /// Create a failing check entry with an optional fix command.
    pub fn fail(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
        fix: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            fix,
        }
    }
}

// ── DoctorSummary ─────────────────────────────────────────────────────

/// Summary counts for a doctor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoctorSummary {
    /// Number of passing checks.
    pub pass: usize,
    /// Number of warning checks.
    pub warn: usize,
    /// Number of failing checks.
    pub fail: usize,
}

impl DoctorSummary {
    /// Create a summary from an iterator of check entries.
    pub fn from_checks<'a>(checks: impl IntoIterator<Item = &'a CheckEntry>) -> Self {
        let mut pass = 0;
        let mut warn = 0;
        let mut fail = 0;
        for c in checks {
            match c.status {
                CheckStatus::Pass => pass += 1,
                CheckStatus::Warn => warn += 1,
                CheckStatus::Fail => fail += 1,
            }
        }
        Self { pass, warn, fail }
    }

    /// Returns `true` if all checks passed (no warnings or failures).
    pub fn is_healthy(&self) -> bool {
        self.warn == 0 && self.fail == 0
    }

    /// Returns `true` if there are any failures (blocking).
    pub fn has_failures(&self) -> bool {
        self.fail > 0
    }

    /// Returns `true` if there are any issues (warnings or failures).
    pub fn has_issues(&self) -> bool {
        self.warn > 0 || self.fail > 0
    }
}

// ── DoctorReport ──────────────────────────────────────────────────────

/// Full diagnostic report from a doctor run.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Tool name that produced this report (e.g., `"wai"`).
    pub tool: String,
    /// All check results.
    pub checks: Vec<CheckEntry>,
    /// Aggregate summary.
    pub summary: DoctorSummary,
}

impl DoctorReport {
    /// Create a new doctor report.
    pub fn new(tool: impl Into<String>, checks: Vec<CheckEntry>) -> Self {
        let summary = DoctorSummary::from_checks(&checks);
        Self {
            tool: tool.into(),
            checks,
            summary,
        }
    }

    /// Returns `true` if all checks passed.
    pub fn is_healthy(&self) -> bool {
        self.summary.is_healthy()
    }

    /// Returns the exit code for this report (0 = healthy, 1 = failures).
    pub fn exit_code(&self) -> i32 {
        if self.summary.has_failures() { 1 } else { 0 }
    }

    /// Serialize to a JSON envelope with kind `Doctor`.
    ///
    /// `cli_version` must be the caller's own CLI version (see
    /// [`crate::envelope::Envelope::success`]).
    pub fn to_envelope(&self, cli_version: &str) -> Envelope<&Self> {
        let hints = self
            .checks
            .iter()
            .filter_map(|c| {
                c.fix.as_ref().map(|fix| crate::envelope::HintEntry {
                    command: fix.clone(),
                    description: c.message.clone(),
                })
            })
            .collect::<Vec<_>>();

        let warnings = self
            .checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Warn))
            .map(|c| crate::envelope::Warning {
                rule_name: c.name.clone(),
                entity_id: None,
                message: c.message.clone(),
                suggested_remediation: c.fix.clone(),
            })
            .collect();

        Envelope::success(cli_version, EnvelopeKind::Doctor, self, warnings, hints)
    }
}

// ── DoctorCheck trait ─────────────────────────────────────────────────

/// A single diagnostic check that can be run by the doctor.
///
/// Every check provides [`name`], [`description`], and [`run`].
/// Optionally, a check can support auto-fix via [`auto_fixable`] and [`fix`].
///
/// # Examples
///
/// ```rust
/// use genesis::doctor::DoctorCheck;
/// use genesis::suite_linter::{LintResult, Severity};
/// use std::path::Path;
///
/// struct ConfigExists;
///
/// impl DoctorCheck for ConfigExists {
///     fn name(&self) -> &'static str { "config-exists" }
///     fn description(&self) -> &'static str { "Check that config.toml exists" }
///
///     fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
///         let config_path = repo_root.join("config.toml");
///         if config_path.exists() {
///             Ok(vec![]) // pass
///         } else {
///             Ok(vec![LintResult::with_fix(
///                 "config.toml not found",
///                 Severity::Error,
///                 "tool init",
///             )])
///         }
///     }
///
///     fn auto_fixable(&self) -> bool { true }
///
///     fn fix(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
///         // Create default config
///         std::fs::write(repo_root.join("config.toml"), "key = \"value\"\n")?;
///         Ok(vec![])
///     }
/// }
/// ```
pub trait DoctorCheck: Send + Sync {
    /// Unique name for this check (e.g., `"wai.directory-structure"`).
    fn name(&self) -> &'static str;

    /// Human-readable description of what this check validates.
    fn description(&self) -> &'static str;

    /// Run the check against the given repo root.
    ///
    /// Return an empty `Vec` on pass, or one or more [`LintResult`]s on issue.
    /// Errors are caught by the runner and reported as a single error result.
    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>>;

    /// Whether this check supports automatic fixing via [`fix`].
    ///
    /// Default: `false`.
    fn auto_fixable(&self) -> bool {
        false
    }

    /// Apply the automatic fix for this check.
    ///
    /// Only called if [`auto_fixable`] returns `true`.
    /// Default: returns an empty Vec (no-op).
    fn fix(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let _ = repo_root;
        Ok(vec![])
    }
}

// ── DoctorRunner ─────────────────────────────────────────────────────

/// Orchestrates running a collection of [`DoctorCheck`]s and producing
/// a [`DoctorReport`].
///
/// Handles check execution, error catching, auto-fix dispatch, and
/// report construction.
///
/// # Examples
///
/// ```rust
/// use genesis::doctor::{DoctorCheck, DoctorRunner};
/// use genesis::suite_linter::{LintResult, Severity};
/// use std::path::Path;
///
/// struct PassCheck;
/// impl DoctorCheck for PassCheck {
///     fn name(&self) -> &'static str { "pass" }
///     fn description(&self) -> &'static str { "Always passes" }
///     fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
///         Ok(vec![])
///     }
/// }
///
/// let runner = DoctorRunner::new(vec![Box::new(PassCheck)]);
/// let report = runner.run(Path::new("/tmp"), false).unwrap();
/// assert!(report.is_healthy());
/// ```
pub struct DoctorRunner {
    tool_name: String,
    checks: Vec<Box<dyn DoctorCheck>>,
}

impl DoctorRunner {
    /// Create a new runner with the given checks.
    pub fn new(checks: Vec<Box<dyn DoctorCheck>>) -> Self {
        Self {
            tool_name: "doctor".to_string(),
            checks,
        }
    }

    /// Set the tool name (fluent). Default: `"doctor"`.
    pub fn with_tool_name(mut self, name: impl Into<String>) -> Self {
        self.tool_name = name.into();
        self
    }

    /// Add a check to the runner (fluent).
    pub fn with(mut self, check: Box<dyn DoctorCheck>) -> Self {
        self.checks.push(check);
        self
    }

    /// Register a check (mutable, for builder pattern).
    pub fn register(&mut self, check: Box<dyn DoctorCheck>) {
        self.checks.push(check);
    }

    /// Number of registered checks.
    pub fn len(&self) -> usize {
        self.checks.len()
    }

    /// Check if the runner has no registered checks.
    pub fn is_empty(&self) -> bool {
        self.checks.is_empty()
    }

    /// Run all checks and produce a report.
    ///
    /// If `fix` is `true`, auto-fixable checks will have their [`DoctorCheck::fix`]
    /// method called before the report is generated. Fix results appear as
    /// check entries with the post-fix status.
    ///
    /// When `fix` is `true` and a check passes after fixing, it is reported
    /// as a passing check with a message indicating the fix was applied.
    ///
    /// # Errors
    ///
    /// This function does not return `Err` for individual check failures —
    /// those are captured and reported as fail entries in the [`DoctorReport`].
    /// `Err` is only returned for systemic failures (e.g., filesystem errors
    /// in the fix path that cannot be caught).
    pub fn run(
        &self,
        repo_root: &Path,
        fix: bool,
    ) -> Result<DoctorReport, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();

        for check in &self.checks {
            if fix && check.auto_fixable() {
                // Run check first
                let pre_results = match check.run(repo_root) {
                    Ok(r) => r,
                    Err(e) => {
                        entries.push(CheckEntry::fail(
                            check.name(),
                            check.description(),
                            format!("pre-fix check failed: {}", e),
                            None,
                        ));
                        continue;
                    }
                };

                let has_issues = pre_results.iter().any(|r| r.severity != Severity::Advisory);

                if has_issues {
                    // Apply fix
                    let fix_results = match check.fix(repo_root) {
                        Ok(r) => r,
                        Err(e) => {
                            entries.push(CheckEntry::fail(
                                check.name(),
                                check.description(),
                                format!("fix failed: {}", e),
                                None,
                            ));
                            continue;
                        }
                    };

                    // Run check again to verify fix
                    let post_results = match check.run(repo_root) {
                        Ok(r) => r,
                        Err(e) => {
                            entries.push(CheckEntry::fail(
                                check.name(),
                                check.description(),
                                format!("post-fix check failed: {}", e),
                                None,
                            ));
                            continue;
                        }
                    };

                    let still_has_issues = post_results
                        .iter()
                        .any(|r| r.severity != Severity::Advisory);

                    if still_has_issues {
                        // Collect all failing messages
                        let msgs: Vec<String> = post_results
                            .iter()
                            .filter(|r| r.severity != Severity::Advisory)
                            .map(|r| r.message.clone())
                            .collect();
                        entries.push(CheckEntry::fail(
                            check.name(),
                            check.description(),
                            format!("fix applied but still has issues: {}", msgs.join("; ")),
                            fix_results.first().and_then(|r| r.fix.clone()),
                        ));
                    } else if fix_results.is_empty() {
                        entries.push(CheckEntry::pass(
                            check.name(),
                            check.description(),
                            "fixed".to_string(),
                        ));
                    } else {
                        // Use the fix result message if present
                        let msg = fix_results
                            .first()
                            .map(|r| format!("fixed: {}", r.message))
                            .unwrap_or_else(|| "fixed".to_string());
                        entries.push(CheckEntry::pass(check.name(), check.description(), msg));
                    }
                } else {
                    entries.push(CheckEntry::pass(
                        check.name(),
                        check.description(),
                        "no issues found".to_string(),
                    ));
                }
            } else {
                // Normal (no fix) path
                let results = match check.run(repo_root) {
                    Ok(r) => r,
                    Err(e) => {
                        entries.push(CheckEntry::fail(
                            check.name(),
                            check.description(),
                            format!("check failed: {}", e),
                            None,
                        ));
                        continue;
                    }
                };

                if results.is_empty() {
                    entries.push(CheckEntry::pass(
                        check.name(),
                        check.description(),
                        "no issues found".to_string(),
                    ));
                } else {
                    for result in results {
                        let status = match result.severity {
                            Severity::Advisory => CheckStatus::Warn,
                            Severity::Warning => CheckStatus::Warn,
                            Severity::Error => CheckStatus::Fail,
                        };
                        entries.push(CheckEntry {
                            name: check.name().to_string(),
                            description: check.description().to_string(),
                            status,
                            message: result.message,
                            fix: result.fix,
                        });
                    }
                }
            }
        }

        Ok(DoctorReport::new(&self.tool_name, entries))
    }
}

// ── Convenience builders ──────────────────────────────────────────────

/// Adapter that wraps a [`LintCheck`](crate::suite_linter::LintCheck) as a [`DoctorCheck`].
///
/// Created via [`doctor::lint_to_doctor`]. The adapter delegates `run()`
/// to the wrapped check and has no auto-fix support.
pub struct LintCheckAdapter<C: crate::suite_linter::LintCheck>(pub C);

impl<C: crate::suite_linter::LintCheck + 'static> DoctorCheck for LintCheckAdapter<C> {
    fn name(&self) -> &'static str {
        self.0.name()
    }

    fn description(&self) -> &'static str {
        self.0.description()
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        self.0.run(repo_root)
    }
}

/// Create a [`DoctorRunner`] from a slice of [`DoctorCheck`] trait objects.
///
/// Convenience wrapper when you already have a `Vec<Box<dyn DoctorCheck>>`.
pub fn runner(checks: Vec<Box<dyn DoctorCheck>>) -> DoctorRunner {
    DoctorRunner::new(checks)
}

/// Wrap an existing [`LintCheck`](crate::suite_linter::LintCheck) as a [`DoctorCheck`] (no auto-fix support).
///
/// This lets tools that already implement `LintCheck` use the doctor
/// framework without rewriting their checks.
pub fn lint_to_doctor<C: crate::suite_linter::LintCheck + 'static>(
    check: C,
) -> LintCheckAdapter<C> {
    LintCheckAdapter(check)
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(test)]
#[path = "doctor/tests.rs"]
mod tests;
