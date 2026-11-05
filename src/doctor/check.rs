//! Doctor check trait and adapters.

use crate::suite_linter::LintResult;
use std::path::Path;

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

// ── Adapter ───────────────────────────────────────────────────────────

/// Adapter that wraps a [`LintCheck`](crate::suite_linter::LintCheck) as a [`DoctorCheck`].
///
/// Created via [`lint_to_doctor`]. The adapter delegates `run()`
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

/// Wrap an existing [`LintCheck`](crate::suite_linter::LintCheck) as a [`DoctorCheck`] (no auto-fix support).
///
/// This lets tools that already implement `LintCheck` use the doctor
/// framework without rewriting their checks.
pub fn lint_to_doctor<C: crate::suite_linter::LintCheck + 'static>(
    check: C,
) -> LintCheckAdapter<C> {
    LintCheckAdapter(check)
}
