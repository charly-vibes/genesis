//! `LinterRegistry` behavior tests (split from tests.rs for the pretender
//! file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::tests_support::{FixedCheck, test_root};
use super::*;

// ── LinterRegistry ────────────────────────────────────────────────

#[test]
fn test_registry_registration() {
    let empty = LinterRegistry::new();
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);

    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "passing",
        description: "Always passes",
        results: Vec::new(),
    }));
    assert_eq!(reg.len(), 1);

    reg.register_all(vec![
        Box::new(FixedCheck {
            name: "warning",
            description: "Always warns",
            results: vec![LintResult::new(
                "something looks suspicious",
                Severity::Warning,
            )],
        }),
        Box::new(FixedCheck {
            name: "error",
            description: "Always errors",
            results: vec![LintResult::new("config file is missing", Severity::Error)],
        }),
    ]);
    assert_eq!(reg.len(), 3);

    let names = reg.check_names();
    assert!(names.contains(&"passing"), "names: {names:?}");
    assert!(names.contains(&"warning"), "names: {names:?}");
    assert!(names.contains(&"error"), "names: {names:?}");
}

#[test]
fn test_registry_find() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "passing",
        description: "Always passes",
        results: Vec::new(),
    }));
    let found = reg.find("passing");
    assert!(found.is_some());
    assert_eq!(found.require("value").name(), "passing");
    assert!(reg.find("nonexistent").is_none());
}

#[test]
fn test_registry_run_all() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "passing",
        description: "Always passes",
        results: Vec::new(),
    }));
    reg.register(Box::new(FixedCheck {
        name: "warning",
        description: "Always warns",
        results: vec![LintResult::new(
            "something looks suspicious",
            Severity::Warning,
        )],
    }));
    reg.register(Box::new(FixedCheck {
        name: "error",
        description: "Always errors",
        results: vec![LintResult::new("config file is missing", Severity::Error)],
    }));

    let results = reg.run_all(&test_root());
    assert_eq!(results.len(), 3);

    let (passing_check, passing_results) = &results[0];
    assert_eq!(passing_check.name(), "passing");
    assert!(passing_results.is_empty());

    let (warning_check, warning_results) = &results[1];
    assert_eq!(warning_check.name(), "warning");
    assert_eq!(warning_results[0].severity, Severity::Warning);

    let (error_check, error_results) = &results[2];
    assert_eq!(error_check.name(), "error");
    assert_eq!(error_results[0].severity, Severity::Error);
}

#[test]
fn test_registry_run_named() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "passing",
        description: "Always passes",
        results: Vec::new(),
    }));
    reg.register(Box::new(FixedCheck {
        name: "error",
        description: "Always errors",
        results: vec![LintResult::new("config file is missing", Severity::Error)],
    }));

    let results = reg.run_named("error", &test_root());
    assert!(results.is_some());
    assert_eq!(results.require("value").len(), 1);
}

#[test]
fn test_registry_run_named_unknown() {
    let reg = LinterRegistry::new();
    assert!(reg.run_named("nonexistent", &test_root()).is_none());
}

#[test]
fn test_registry_run_named_panicking_check() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "panic",
        description: "Always panics",
        results: Vec::new(),
    }));

    let results = reg.run_named("panic", &test_root());
    assert!(results.is_some());
    let results = results.require("value");
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
    reg.register(Box::new(FixedCheck {
        name: "warning",
        description: "Always warns",
        results: vec![LintResult::new(
            "something looks suspicious",
            Severity::Warning,
        )],
    }));
    reg.register(Box::new(FixedCheck {
        name: "error",
        description: "Always errors",
        results: vec![LintResult::new("config file is missing", Severity::Error)],
    }));

    let results = reg.run_filtered(&test_root(), Severity::Warning);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_registry_run_filtered_excludes_advisory() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "advisory",
        description: "Always advisory",
        results: vec![LintResult::new("info", Severity::Advisory)],
    }));
    reg.register(Box::new(FixedCheck {
        name: "error",
        description: "Always errors",
        results: vec![LintResult::new("config file is missing", Severity::Error)],
    }));

    let results = reg.run_filtered(&test_root(), Severity::Warning);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Error);
}

#[test]
fn test_registry_run_filtered_only_errors() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "warning",
        description: "Always warns",
        results: vec![LintResult::new(
            "something looks suspicious",
            Severity::Warning,
        )],
    }));
    reg.register(Box::new(FixedCheck {
        name: "error",
        description: "Always errors",
        results: vec![LintResult::new("config file is missing", Severity::Error)],
    }));

    let results = reg.run_filtered(&test_root(), Severity::Error);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Error);
}

#[test]
fn test_registry_run_all_handles_panicking_check() {
    let mut reg = LinterRegistry::new();
    reg.register(Box::new(FixedCheck {
        name: "passing",
        description: "Always passes",
        results: Vec::new(),
    }));
    reg.register(Box::new(FixedCheck {
        name: "panic",
        description: "Always panics",
        results: Vec::new(),
    }));

    let results = reg.run_all(&test_root());
    assert_eq!(results.len(), 2);

    // The panicking check should be caught and reported as an error
    let (name, check_results) = &results[1];
    assert_eq!(name.name(), "panic");
    assert_eq!(check_results[0].severity, Severity::Error);
    assert!(check_results[0].message.contains("check failed"));
}
