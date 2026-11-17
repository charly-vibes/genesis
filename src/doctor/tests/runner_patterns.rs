//! `DoctorRunner` pass/warn/fail pattern tests.

use super::*;

/// `(check, expected_healthy, expected_pass, expected_warn, expected_fail)`.
type Case = (Box<dyn DoctorCheck>, bool, usize, usize, usize);

#[test]
fn runner_summaries_reflect_check_outcomes() {
    let cases: Vec<Case> = vec![
        (Box::new(PassCheck), true, 1, 0, 0),
        (Box::new(WarnCheck), false, 0, 1, 0),
        (Box::new(FailCheck), false, 0, 0, 1),
    ];
    for (check, healthy, pass, warn, fail) in cases {
        let dir = tmp();
        let runner = DoctorRunner::new(vec![check]);
        let report = runner.run(dir.path(), false).require("value");
        assert_eq!(report.is_healthy(), healthy, "{report:?}");
        assert_eq!(report.summary.pass, pass, "{report:?}");
        assert_eq!(report.summary.warn, warn, "{report:?}");
        assert_eq!(report.summary.fail, fail, "{report:?}");
    }
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
