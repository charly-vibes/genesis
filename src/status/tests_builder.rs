//! StatusBuilder + DoctorStatusBridge tests (split from tests.rs for the
//! pretender lazy-cluster and file-lines budget).

use super::*;
use crate::doctor::{DoctorCheck, DoctorRunner};
use crate::suite_linter::{LintResult, Severity};
use crate::testutil::TestUnwrap;
use std::path::Path;

struct WarnContributor;
impl StatusContributor for WarnContributor {
    fn name(&self) -> &'static str {
        "warn"
    }
    fn status(&self, _: &Path) -> Result<StatusSection, String> {
        Ok(StatusSection {
            tool: "warn".into(),
            level: StatusLevel::Warning,
            summary: "1 warning".into(),
            items: vec![StatusItem::warning("config", "deprecated field")],
            suggestions: vec!["run doctor".into()],
        })
    }
}

struct ErrorContributor;
impl StatusContributor for ErrorContributor {
    fn name(&self) -> &'static str {
        "error"
    }
    fn status(&self, _: &Path) -> Result<StatusSection, String> {
        Ok(StatusSection {
            tool: "error".into(),
            level: StatusLevel::Error,
            summary: "not initialized".into(),
            items: vec![StatusItem::error("state", "missing")],
            suggestions: vec!["error init".into()],
        })
    }
}

struct UnreachableContributor;
impl StatusContributor for UnreachableContributor {
    fn name(&self) -> &'static str {
        "unreachable"
    }
    fn status(&self, _: &Path) -> Result<StatusSection, String> {
        Err("not initialized".into())
    }
}

struct PassContributor;
impl StatusContributor for PassContributor {
    fn name(&self) -> &'static str {
        "pass"
    }
    fn status(&self, _: &Path) -> Result<StatusSection, String> {
        Ok(StatusSection {
            tool: "pass".into(),
            level: StatusLevel::Healthy,
            summary: "all good".into(),
            items: vec![StatusItem::healthy("config", "present")],
            suggestions: vec![],
        })
    }
}

// ── StatusBuilder ─────────────────────────────────────────────────

// ── StatusBuilder ─────────────────────────────────────────────────

#[test]
fn test_builder_empty() {
    let builder = StatusBuilder::new();
    assert!(builder.is_empty());
    let report = builder.build(Path::new("/tmp")).require("value");
    assert!(report.is_healthy());
    assert_eq!(report.sections.len(), 0);
}

#[test]
fn test_builder_single() {
    let mut builder = StatusBuilder::new();
    builder.register(Box::new(PassContributor));
    let report = builder.build(Path::new("/tmp")).require("value");
    assert_eq!(report.sections.len(), 1);
    assert!(report.is_healthy());
}

#[test]
fn test_builder_mixed() {
    let mut builder = StatusBuilder::new();
    builder.register(Box::new(PassContributor));
    builder.register(Box::new(WarnContributor));
    let report = builder.build(Path::new("/tmp")).require("value");
    assert_eq!(report.sections.len(), 2);
    assert!(!report.is_healthy());
    assert!(!report.has_errors());
}

#[test]
fn test_builder_with_errors() {
    let mut builder = StatusBuilder::new();
    builder.register(Box::new(PassContributor));
    builder.register(Box::new(ErrorContributor));
    let report = builder.build(Path::new("/tmp")).require("value");
    assert_eq!(report.sections.len(), 2);
    assert!(report.has_errors());
}

#[test]
fn test_builder_unreachable_contributor() {
    let mut builder = StatusBuilder::new();
    builder.register(Box::new(UnreachableContributor));
    let report = builder.build(Path::new("/tmp")).require("value");
    assert_eq!(report.sections.len(), 1);
    assert!(report.has_errors());
    assert!(report.sections[0].summary.contains("unreachable"));
}

// ── DoctorStatusBridge ────────────────────────────────────────────

#[test]
fn test_doctor_bridge_healthy() {
    struct PassCheck;
    impl DoctorCheck for PassCheck {
        fn name(&self) -> &'static str {
            "pass"
        }
        fn description(&self) -> &'static str {
            "passes"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![])
        }
    }

    let dir = tempfile::tempdir().require("value");
    let runner = DoctorRunner::new(vec![Box::new(PassCheck)]);
    let bridge = DoctorStatusBridge::new("test-tool", runner);
    let section = bridge.status(dir.path()).require("value");
    assert_eq!(section.level, StatusLevel::Healthy);
    assert!(section.summary.contains("all passed"));
}

#[test]
fn test_doctor_bridge_with_issues() {
    struct FailCheck;
    impl DoctorCheck for FailCheck {
        fn name(&self) -> &'static str {
            "fail"
        }
        fn description(&self) -> &'static str {
            "fails"
        }
        fn run(&self, _: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
            Ok(vec![LintResult::with_fix(
                "broken",
                Severity::Error,
                "fix it",
            )])
        }
    }

    let dir = tempfile::tempdir().require("value");
    let runner = DoctorRunner::new(vec![Box::new(FailCheck)]);
    let bridge = DoctorStatusBridge::new("test-tool", runner);
    let section = bridge.status(dir.path()).require("value");
    assert_eq!(section.level, StatusLevel::Error);
    assert_eq!(section.items.len(), 1);
    assert_eq!(section.suggestions.len(), 1);
    assert_eq!(section.suggestions[0], "fix it");
}
