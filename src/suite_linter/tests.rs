//! Unit tests for `suite_linter` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;
use std::path::{Path, PathBuf};

// ── Helpers ───────────────────────────────────────────────────────

/// One parameterized mock check replaces five structurally identical mock
/// impls (each with name/description/run methods) — pretender flags
/// clusters of identical-shaped units.
struct FixedCheck {
    name: &'static str,
    description: &'static str,
    results: Vec<LintResult>,
}

impl LintCheck for FixedCheck {
    fn name(&self) -> &'static str {
        self.name
    }
    fn description(&self) -> &'static str {
        self.description
    }
    fn run(&self, _repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        if self.name == "panic" {
            // The panic mock reports an internal failure instead of results.
            return Err("internal failure".into());
        }
        Ok(self.results.clone())
    }
}

fn test_root() -> PathBuf {
    let root = std::env::temp_dir().join("genesis-suite-linter-test");
    assert!(
        !root.join(".git").exists(),
        "test root must not live inside a git repository"
    );
    root
}

// ── LintCheck ─────────────────────────────────────────────────────

trait RunOutput {
    fn run_output(&self) -> Vec<LintResult>;
}

impl<C: LintCheck> RunOutput for C {
    fn run_output(&self) -> Vec<LintResult> {
        self.run(&std::env::temp_dir().join("genesis-suite-linter-test"))
            .require("run check")
    }
}

#[test]
fn test_check_outputs_match_declared_severity() {
    let cases = [
        (
            FixedCheck {
                name: "passing",
                description: "Always passes",
                results: Vec::new(),
            }
            .run_output(),
            0,
            None,
            "passing",
        ),
        (
            FixedCheck {
                name: "warning",
                description: "Always warns",
                results: vec![LintResult::new(
                    "something looks suspicious",
                    Severity::Warning,
                )],
            }
            .run_output(),
            1,
            Some(Severity::Warning),
            "warning",
        ),
        (
            FixedCheck {
                name: "error",
                description: "Always errors",
                results: vec![LintResult::new("config file is missing", Severity::Error)],
            }
            .run_output(),
            1,
            Some(Severity::Error),
            "error",
        ),
    ];
    for (results, expected_len, severity, name) in cases {
        assert_eq!(results.len(), expected_len, "{name}");
        if let Some(severity) = severity {
            assert_eq!(results[0].severity, severity, "{name}");
        }
    }
}

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

// ── EvalsGuidelinesAdoption (add-evals-guidelines) ────────────────
