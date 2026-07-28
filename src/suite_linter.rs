//! Suite-wide lint orchestrator.
//!
//! The suite_linter is an **orchestrator, not a monolith**. Each tool defines
//! its own checks via the `LintCheck` trait; genesis just runs them.
//!
//! - `LintCheck` trait — tools implement this for each check
//! - `LintResult` — severity + message + optional fix command
//! - `LinterRegistry` — tools register checks, genesis runs them

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
}
