//! Doctor-to-status bridge: surface a [`crate::doctor::DoctorRunner`] as a
//! [`StatusContributor`] so existing doctor setups appear in the dashboard.

use std::path::Path;

use super::{StatusItem, StatusLevel, StatusSection, StatusContributor};

/// Wrap a [`crate::doctor::DoctorRunner`] as a [`StatusContributor`].
///
/// This lets any tool that already has a `DoctorRunner` automatically
/// surface its health in the status dashboard — no extra work.
///
/// # Example
///
/// ```rust,no_run
/// use genesis::status::StatusBuilder;
/// use genesis::doctor::DoctorRunner;
/// use genesis::status::DoctorStatusBridge;
///
/// let mut builder = StatusBuilder::new();
///
/// // Register a tool's doctor as a status contributor
/// let runner: DoctorRunner = DoctorRunner::new(vec![]);
/// builder.register(Box::new(DoctorStatusBridge::new("my-tool", runner)));
/// ```
pub struct DoctorStatusBridge {
    name: &'static str,
    runner: crate::doctor::DoctorRunner,
}

impl DoctorStatusBridge {
    /// Create a bridge that wraps a `DoctorRunner` for the given tool.
    pub fn new(name: &'static str, runner: crate::doctor::DoctorRunner) -> Self {
        Self { name, runner }
    }
}

impl StatusContributor for DoctorStatusBridge {
    fn name(&self) -> &'static str {
        self.name
    }

    fn status(&self, repo_root: &Path) -> Result<StatusSection, String> {
        let report = self
            .runner
            .run(repo_root, false)
            .map_err(|e| e.to_string())?;

        let items: Vec<StatusItem> = report
            .checks
            .iter()
            .filter(|check| check.status != crate::doctor::CheckStatus::Pass)
            .map(|check| StatusItem {
                label: check.name.clone(),
                value: check.message.clone(),
                level: bridge_level(check.status),
            })
            .collect();
        let summary = doctor_summary(&report);
        let section = StatusSection::with_items(self.name, summary, items)
            .with_fix_suggestions(&report.checks);
        Ok(section)
    }
}

/// Map a doctor check status to a dashboard status level (pass is filtered
/// out before this is called).
fn bridge_level(status: crate::doctor::CheckStatus) -> StatusLevel {
    match status {
        crate::doctor::CheckStatus::Pass => StatusLevel::Healthy,
        crate::doctor::CheckStatus::Warn => StatusLevel::Warning,
        crate::doctor::CheckStatus::Fail => StatusLevel::Error,
    }
}

/// One-line summary of a doctor report.
fn doctor_summary(report: &crate::doctor::DoctorReport) -> String {
    if report.is_healthy() {
        format!("{} checks, all passed", report.summary.pass)
    } else {
        format!(
            "{} checks, {} pass, {} warn, {} fail",
            report.checks.len(),
            report.summary.pass,
            report.summary.warn,
            report.summary.fail,
        )
    }
}

/// Trait extension: attach each failing check's fix as a suggestion.
trait WithFixSuggestions {
    fn with_fix_suggestions(self, checks: &[crate::doctor::CheckEntry]) -> Self;
}

impl WithFixSuggestions for StatusSection {
    fn with_fix_suggestions(mut self, checks: &[crate::doctor::CheckEntry]) -> Self {
        for check in checks {
            if let Some(ref fix) = check.fix {
                self = self.with_suggestion(fix.clone());
            }
        }
        self
    }
}
