//! Doctor runner: executes checks, dispatches auto-fix, builds the report.

use super::check::DoctorCheck;
use super::report::{CheckEntry, DoctorReport};
use crate::suite_linter::{LintResult, Severity};
use std::path::Path;

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
                self.run_fixable_check(check.as_ref(), repo_root, &mut entries);
            } else {
                self.run_plain_check(check.as_ref(), repo_root, &mut entries);
            }
        }

        Ok(DoctorReport::new(&self.tool_name, entries))
    }

    /// Run an auto-fixable check: pre-check, fix, post-check, verify.
    fn run_fixable_check(
        &self,
        check: &dyn DoctorCheck,
        repo_root: &Path,
        entries: &mut Vec<CheckEntry>,
    ) {
        // Run check first
        let Some(pre_results) = self.try_run(check, repo_root, "pre-fix check failed", entries)
        else {
            return;
        };

        if pre_results.iter().all(|r| r.severity == Severity::Advisory) {
            self.push_pass(check, entries, "no issues found");
            return;
        }

        // Apply fix
        let Some(fix_results) = self.try_fix(check, repo_root, entries) else {
            return;
        };

        // Run check again to verify fix
        let Some(post_results) = self.try_run(check, repo_root, "post-fix check failed", entries)
        else {
            return;
        };

        entries.push(verify_fix_entry(check, &post_results, &fix_results));
    }

    /// Run a check without auto-fix, translating results to entries.
    fn run_plain_check(
        &self,
        check: &dyn DoctorCheck,
        repo_root: &Path,
        entries: &mut Vec<CheckEntry>,
    ) {
        let Some(results) = self.try_run(check, repo_root, "check failed", entries) else {
            return;
        };

        if results.is_empty() {
            self.push_pass(check, entries, "no issues found");
            return;
        }

        for result in results {
            entries.push(entry_from_lint(check, result));
        }
    }

    /// Push a passing entry for `check`.
    fn push_pass(&self, check: &dyn DoctorCheck, entries: &mut Vec<CheckEntry>, msg: &str) {
        entries.push(CheckEntry::pass(
            check.name(),
            check.description(),
            msg.to_string(),
        ));
    }

    /// Run a check, pushing a fail entry (named `err_context`) on error.
    fn try_run(
        &self,
        check: &dyn DoctorCheck,
        repo_root: &Path,
        err_context: &str,
        entries: &mut Vec<CheckEntry>,
    ) -> Option<Vec<LintResult>> {
        match check.run(repo_root) {
            Ok(r) => Some(r),
            Err(e) => {
                entries.push(CheckEntry::fail(
                    check.name(),
                    check.description(),
                    format!("{err_context}: {e}"),
                    None,
                ));
                None
            }
        }
    }

    /// Apply a check's fix, pushing a fail entry on error.
    fn try_fix(
        &self,
        check: &dyn DoctorCheck,
        repo_root: &Path,
        entries: &mut Vec<CheckEntry>,
    ) -> Option<Vec<LintResult>> {
        match check.fix(repo_root) {
            Ok(r) => Some(r),
            Err(e) => {
                entries.push(CheckEntry::fail(
                    check.name(),
                    check.description(),
                    format!("fix failed: {e}"),
                    None,
                ));
                None
            }
        }
    }
}

/// Entry describing the post-fix state of a check.
fn verify_fix_entry(
    check: &dyn DoctorCheck,
    post_results: &[LintResult],
    fix_results: &[LintResult],
) -> CheckEntry {
    let still_has_issues = post_results
        .iter()
        .any(|r| r.severity != Severity::Advisory);

    if still_has_issues {
        let fix = fix_results.first().and_then(|r| r.fix.clone());
        CheckEntry::fail(
            check.name(),
            check.description(),
            format!(
                "fix applied but still has issues: {}",
                failing_messages(post_results).join("; ")
            ),
            fix,
        )
    } else if fix_results.is_empty() {
        CheckEntry::pass(check.name(), check.description(), "fixed".to_string())
    } else {
        // Use the fix result message if present
        let msg = fix_results
            .first()
            .map(|r| format!("fixed: {}", r.message))
            .unwrap_or_else(|| "fixed".to_string());
        CheckEntry::pass(check.name(), check.description(), msg)
    }
}

/// Translate a lint result into a report entry for `check`.
fn entry_from_lint(check: &dyn DoctorCheck, result: LintResult) -> CheckEntry {
    let status = to_check_status(result.severity);
    CheckEntry {
        name: check.name().to_string(),
        description: check.description().to_string(),
        status,
        message: result.message,
        fix: result.fix,
    }
}

/// Messages of all non-advisory results.
fn failing_messages(results: &[LintResult]) -> Vec<String> {
    results
        .iter()
        .filter(|r| r.severity != Severity::Advisory)
        .map(|r| r.message.clone())
        .collect()
}

/// Map lint severity to report status.
fn to_check_status(severity: Severity) -> super::report::CheckStatus {
    match severity {
        Severity::Advisory | Severity::Warning => super::report::CheckStatus::Warn,
        Severity::Error => super::report::CheckStatus::Fail,
    }
}

/// Create a [`DoctorRunner`] from a slice of [`DoctorCheck`] trait objects.
///
/// Convenience wrapper when you already have a `Vec<Box<dyn DoctorCheck>>`.
pub fn runner(checks: Vec<Box<dyn DoctorCheck>>) -> DoctorRunner {
    DoctorRunner::new(checks)
}
