//! Unit tests for `suite_linter` check semantics and result formatting
//! (extracted from the inline `mod tests`; registry tests live in
//! tests_registry.rs, filesystem tests in tests_drift.rs).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::tests_support::FixedCheck;
use super::*;

// ── Helpers ───────────────────────────────────────────────────────

trait RunOutput {
    fn run_output(&self) -> Vec<LintResult>;
}

impl<C: LintCheck> RunOutput for C {
    fn run_output(&self) -> Vec<LintResult> {
        self.run(&std::env::temp_dir().join("genesis-suite-linter-test"))
            .require("run check")
    }
}

// ── LintCheck ─────────────────────────────────────────────────────

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
