//! Unit tests for `doctor` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use crate::suite_linter::Severity;
use std::path::PathBuf;
use tempfile::TempDir;

// ── Helpers ───────────────────────────────────────────────────────

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

struct PassCheck;
impl DoctorCheck for PassCheck {
    fn name(&self) -> &'static str {
        "pass"
    }
    fn description(&self) -> &'static str {
        "Always passes"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        Ok(vec![])
    }
}

struct WarnCheck;
impl DoctorCheck for WarnCheck {
    fn name(&self) -> &'static str {
        "warn"
    }
    fn description(&self) -> &'static str {
        "Always warns"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        Ok(vec![LintResult::new("looks suspicious", Severity::Warning)])
    }
}

struct FailCheck;
impl DoctorCheck for FailCheck {
    fn name(&self) -> &'static str {
        "fail"
    }
    fn description(&self) -> &'static str {
        "Always fails"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        Ok(vec![LintResult::new("config missing", Severity::Error)])
    }
}

struct FailWithFixCheck;
impl DoctorCheck for FailWithFixCheck {
    fn name(&self) -> &'static str {
        "fail-fix"
    }
    fn description(&self) -> &'static str {
        "Fails but has fix command"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        Ok(vec![LintResult::with_fix(
            "config missing",
            Severity::Error,
            "tool init",
        )])
    }
}

struct PanicCheck;
impl DoctorCheck for PanicCheck {
    fn name(&self) -> &'static str {
        "panic"
    }
    fn description(&self) -> &'static str {
        "Always panics"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        Err("internal failure".into())
    }
}

struct AutoFixableCheck {
    marker_path: PathBuf,
}
impl AutoFixableCheck {
    fn new(dir: &Path) -> Self {
        Self {
            marker_path: dir.join(".fixed-marker"),
        }
    }
}
impl DoctorCheck for AutoFixableCheck {
    fn name(&self) -> &'static str {
        "auto-fixable"
    }
    fn description(&self) -> &'static str {
        "Can auto-fix by creating a marker file"
    }
    fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.marker_path.exists() {
            Ok(vec![])
        } else {
            Ok(vec![LintResult::new(
                "marker file missing",
                Severity::Error,
            )])
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

// ── CheckStatus ───────────────────────────────────────────────────

#[test]
fn test_check_status_pass() {
    assert!(CheckStatus::Pass.is_pass());
    assert!(!CheckStatus::Pass.is_issue());
    assert!(!CheckStatus::Pass.is_fail());
}

#[test]
fn test_check_status_warn() {
    assert!(!CheckStatus::Warn.is_pass());
    assert!(CheckStatus::Warn.is_issue());
    assert!(!CheckStatus::Warn.is_fail());
}

#[test]
fn test_check_status_fail() {
    assert!(!CheckStatus::Fail.is_pass());
    assert!(CheckStatus::Fail.is_issue());
    assert!(CheckStatus::Fail.is_fail());
}

// ── CheckEntry ────────────────────────────────────────────────────

#[test]
fn test_check_entry_pass_builder() {
    let e = CheckEntry::pass("test", "description", "all good");
    assert_eq!(e.name, "test");
    assert_eq!(e.status, CheckStatus::Pass);
    assert!(e.fix.is_none());
}

#[test]
fn test_check_entry_warn_builder() {
    let e = CheckEntry::warn("test", "description", "looks off");
    assert_eq!(e.status, CheckStatus::Warn);
}

#[test]
fn test_check_entry_fail_builder() {
    let e = CheckEntry::fail("test", "description", "broken", Some("fix it".into()));
    assert_eq!(e.status, CheckStatus::Fail);
    assert_eq!(e.fix, Some("fix it".into()));
}

#[test]
fn test_check_entry_fail_without_fix() {
    let e = CheckEntry::fail("test", "description", "broken", None);
    assert_eq!(e.status, CheckStatus::Fail);
    assert!(e.fix.is_none());
}

// ── DoctorSummary ─────────────────────────────────────────────────

#[test]
fn test_summary_from_checks() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::warn("b", "d", "warn"),
        CheckEntry::fail("c", "d", "fail", None),
    ];
    let summary = DoctorSummary::from_checks(&checks);
    assert_eq!(summary.pass, 1);
    assert_eq!(summary.warn, 1);
    assert_eq!(summary.fail, 1);
}

#[test]
fn test_summary_is_healthy() {
    assert!(
        DoctorSummary {
            pass: 1,
            warn: 0,
            fail: 0
        }
        .is_healthy()
    );
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 1,
            fail: 0
        }
        .is_healthy()
    );
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 0,
            fail: 1
        }
        .is_healthy()
    );
}

#[test]
fn test_summary_has_failures() {
    assert!(
        !DoctorSummary {
            pass: 0,
            warn: 1,
            fail: 0
        }
        .has_failures()
    );
    assert!(
        DoctorSummary {
            pass: 0,
            warn: 0,
            fail: 1
        }
        .has_failures()
    );
}

// ── DoctorReport ──────────────────────────────────────────────────

#[test]
fn test_report_from_checks() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::fail("b", "d", "fail", None),
    ];
    let report = DoctorReport::new("test-tool", checks);
    assert_eq!(report.tool, "test-tool");
    assert!(!report.is_healthy());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn test_report_exit_code_healthy() {
    let checks = vec![CheckEntry::pass("a", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn test_report_to_envelope() {
    let checks = vec![
        CheckEntry::pass("a", "d", "ok"),
        CheckEntry::fail("b", "d", "missing config", Some("tool init".into())),
    ];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    assert!(envelope.ok);
    assert_eq!(envelope.envelope_kind, EnvelopeKind::Doctor);
    assert_eq!(envelope.cli_version, "my-tool/1.0.0");
}

#[test]
fn test_report_to_envelope_serializes_json() {
    let checks = vec![CheckEntry::pass("a", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    let json = serde_json::to_string(&envelope).require("value");
    assert!(json.contains("doctor"));
    assert!(json.contains("test"));
    assert!(json.contains("my-tool/1.0.0"));
}

// ── DoctorRunner: pass/warn/fail patterns ─────────────────────────

#[test]
fn test_runner_all_pass() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(PassCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    assert!(report.is_healthy());
    assert_eq!(report.summary.pass, 1);
    assert_eq!(report.summary.fail, 0);
}

#[test]
fn test_runner_with_warning() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(WarnCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    assert!(!report.is_healthy());
    assert_eq!(report.summary.warn, 1);
    assert_eq!(report.summary.pass, 0);
}

#[test]
fn test_runner_with_failure() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(FailCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    assert_eq!(report.summary.fail, 1);
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn test_runner_mixed() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![
        Box::new(PassCheck),
        Box::new(WarnCheck),
        Box::new(FailCheck),
    ]);
    let report = runner.run(dir.path(), false).require("value");
    assert_eq!(report.summary.pass, 1);
    assert_eq!(report.summary.warn, 1);
    assert_eq!(report.summary.fail, 1);
}

#[test]
fn test_runner_multiple_results_from_one_check() {
    struct MultiResultCheck;
    impl DoctorCheck for MultiResultCheck {
        fn name(&self) -> &'static str {
            "multi"
        }
        fn description(&self) -> &'static str {
            "Returns multiple results"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![
                LintResult::new("warning 1", Severity::Warning),
                LintResult::new("error 1", Severity::Error),
                LintResult::new("advisory 1", Severity::Advisory),
            ])
        }
    }

    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(MultiResultCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    // Advisory → Warn, Warning → Warn, Error → Fail
    assert_eq!(report.summary.pass, 0);
    assert_eq!(report.summary.warn, 2); // advisory + warning
    assert_eq!(report.summary.fail, 1); // error
}

#[test]
fn test_fail_with_fix_check() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(FailWithFixCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    assert_eq!(report.summary.fail, 1);
    assert_eq!(report.checks[0].fix.as_deref(), Some("tool init"));
}

#[test]
fn test_runner_panicking_check() {
    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(PanicCheck)]);
    let report = runner.run(dir.path(), false).require("value");
    assert_eq!(report.summary.fail, 1);
    assert!(report.checks[0].message.contains("check failed"));
}

// ── DoctorRunner: fix mode ────────────────────────────────────────

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
    struct NonFixable;
    impl DoctorCheck for NonFixable {
        fn name(&self) -> &'static str {
            "non-fixable"
        }
        fn description(&self) -> &'static str {
            "not fixable"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::new("issue", Severity::Error)])
        }
    }

    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(NonFixable)]);

    // With fix=true but check is not auto-fixable: should not attempt fix
    let report = runner.run(dir.path(), true).require("value");
    assert_eq!(report.summary.fail, 1);
    assert_eq!(report.summary.pass, 0);
}

#[test]
fn test_runner_fix_that_still_fails() {
    struct PartialFix;
    impl DoctorCheck for PartialFix {
        fn name(&self) -> &'static str {
            "partial-fix"
        }
        fn description(&self) -> &'static str {
            "Fix doesn't resolve all issues"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::new("still broken", Severity::Error)])
        }
        fn auto_fixable(&self) -> bool {
            true
        }
        fn fix(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![]) // fix ran but didn't help
        }
    }

    let dir = tmp();
    let runner = DoctorRunner::new(vec![Box::new(PartialFix)]);
    let report = runner.run(dir.path(), true).require("value");
    assert_eq!(report.summary.fail, 1);
    assert!(report.checks[0].message.contains("still has issues"));
}

// ── DoctorRunner: registration ────────────────────────────────────

#[test]
fn test_runner_empty() {
    let runner: DoctorRunner = DoctorRunner::new(vec![]);
    assert!(runner.is_empty());
    assert_eq!(runner.len(), 0);

    let dir = tmp();
    let report = runner.run(dir.path(), false).require("value");
    assert!(report.is_healthy());
}

#[test]
fn test_runner_with_fn() {
    let mut runner = DoctorRunner::new(vec![]);
    runner.register(Box::new(PassCheck));
    assert_eq!(runner.len(), 1);
}

#[test]
fn test_runner_fluent_with() {
    let runner = DoctorRunner::new(vec![])
        .with(Box::new(PassCheck))
        .with(Box::new(WarnCheck));
    assert_eq!(runner.len(), 2);
}

// ── Convenience ───────────────────────────────────────────────────

#[test]
fn test_runner_convenience_fn() {
    let r = runner(vec![Box::new(PassCheck)]);
    assert_eq!(r.len(), 1);
}

// ── DoctorReport serialization ────────────────────────────────────

#[test]
fn test_report_json_serialization() {
    let checks = vec![
        CheckEntry::pass("pass", "d", "ok"),
        CheckEntry::warn("warn", "d", "caution"),
        CheckEntry::fail("fail", "d", "broken", Some("fix".into())),
    ];
    let report = DoctorReport::new("test", checks);
    let json = serde_json::to_string_pretty(&report).require("value");
    assert!(json.contains("\"pass\""));
    assert!(json.contains("\"warn\""));
    assert!(json.contains("\"fail\""));
    assert!(json.contains("\"broken\""));
    assert!(json.contains("\"fix\""));
}

#[test]
fn test_report_json_omits_fix_when_none() {
    let checks = vec![CheckEntry::pass("pass", "d", "ok")];
    let report = DoctorReport::new("test", checks);
    let json = serde_json::to_string(&report).require("value");
    assert!(!json.contains("\"fix\""));
}

#[test]
fn test_report_to_envelope_has_hints_from_fix_commands() {
    let checks = vec![
        CheckEntry::fail("cfg", "d", "missing", Some("tool init".into())),
        CheckEntry::pass("ok", "d", "good"),
    ];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    let hints = envelope.hints.unwrap_or_default();
    assert!(!hints.is_empty());
    assert_eq!(hints[0].command, "tool init");
}

#[test]
fn test_empty_report_to_envelope_has_no_hints() {
    let checks = vec![CheckEntry::pass("ok", "d", "good")];
    let report = DoctorReport::new("test", checks);
    let envelope = report.to_envelope("my-tool/1.0.0");
    assert!(envelope.hints.unwrap_or_default().is_empty());
}
