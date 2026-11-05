//! `DoctorRunner` fix-mode tests.

use super::*;

/// Auto-fixable fixture that creates a marker file when fixed.
pub(super) struct AutoFixableCheck {
    marker_path: std::path::PathBuf,
}
impl AutoFixableCheck {
    pub(super) fn new(dir: &Path) -> Self {
        Self {
            marker_path: dir.join(".fixed-marker"),
        }
    }
}
impl DoctorCheck for AutoFixableCheck {
    check_meta!(
        "auto-fixable",
        "Can auto-fix by creating a marker file"
    );
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.marker_path.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new("marker file missing", Severity::Error)])
        }
    }
    fn auto_fixable(&self) -> bool {
        true
    }
    fn fix(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        std::fs::write(&self.marker_path, "fixed")?;
        Ok(vec![])
    }
}

#[test]
fn test_runner_fix_applies_and_passes() {
    let dir = tmp();
    let check = AutoFixableCheck::new(dir.path());
    let runner = DoctorRunner::new(vec![Box::new(check)]);

    // Before fix: should fail
    let pre_report = runner.run(dir.path(), false).require("value");
    assert_eq!(pre_report.summary.fail, 1);

    // With fix: should apply and verify
    let post_report = runner.run(dir.path(), true).require("value");
    assert!(post_report.is_healthy());
    assert_eq!(post_report.summary.pass, 1);

    // Marker file should exist
    assert!(dir.path().join(".fixed-marker").exists());
}

#[test]
fn test_runner_fix_on_already_healthy_check() {
    let dir = tmp();
    // Create the marker so it passes immediately
    std::fs::write(dir.path().join(".fixed-marker"), "fixed").require("value");
    let check = AutoFixableCheck::new(dir.path());
    let runner = DoctorRunner::new(vec![Box::new(check)]);

    let report = runner.run(dir.path(), true).require("value");
    assert!(report.is_healthy());
    assert_eq!(report.summary.pass, 1);
}

#[test]
fn test_runner_non_fixable_ignores_fix_flag() {
    doctor_fixture!(
        NonFixable,
        "non-fixable",
        "not fixable",
        ok: vec![LintResult::new("issue", Severity::Error)]
    );

    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(NonFixable)]);

    // With fix=true but check is not auto-fixable: should not attempt fix
    let report = runner.run(dir.path(), true).require("value");
    assert_eq!(report.summary.fail, 1);
    assert_eq!(report.summary.pass, 0);
}

#[test]
fn test_runner_fix_that_still_fails() {
    doctor_fixture!(
        PartialFix,
        "partial-fix",
        "Fix doesn't resolve all issues",
        fixable:
        run Ok(vec![LintResult::new("still broken", Severity::Error)]),
        fix Ok(vec![]) // fix ran but didn't help
    );

    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(PartialFix)]);
    let report = runner.run(dir.path(), true).require("value");
    assert_eq!(report.summary.fail, 1);
    assert!(report.checks[0].message.contains("still has issues"));
}
