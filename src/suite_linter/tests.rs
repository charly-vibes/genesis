//! Unit tests for `suite_linter` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

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
    let results = check.run(&test_root()).require("value");
    assert!(results.is_empty());
}

#[test]
fn test_warning_check_returns_warning() {
    let check = WarningCheck;
    let results = check.run(&test_root()).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Warning);
}

#[test]
fn test_error_check_returns_error() {
    let check = ErrorCheck;
    let results = check.run(&test_root()).require("value");
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
    assert_eq!(found.require("value").name(), "passing");
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
    reg.register(Box::new(PanicCheck));

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

// ── EvalsGuidelinesAdoption (add-evals-guidelines) ────────────────

fn write_repo(files: &[(&str, &str)]) -> PathBuf {
    let dir = tempfile::tempdir().require("tempdir");
    for (path, content) in files {
        let full = dir.path().join(path);
        std::fs::create_dir_all(full.parent().require("value")).require("value");
        std::fs::write(full, content).require("value");
    }
    // Leak the temp dir so it outlives the test body.
    let path = dir.path().to_path_buf();
    std::mem::forget(dir);
    path
}

#[test]
fn test_evals_adoption_passes_when_docs_reference_guidelines() {
    let root = write_repo(&[(
        "docs/how-to/evals.md",
        "# Evals\n\nSee the [report contract](../reference/eval-report.md).\n",
    )]);
    let results = EvalsGuidelinesAdoption.run(&root).require("value");
    assert!(results.is_empty(), "expected no findings, got {results:?}");
}

#[test]
fn test_evals_adoption_flags_docs_without_reference() {
    let root = write_repo(&[(
        "docs/how-to/evals.md",
        "# Evals\n\nMy battery runs on pushes.\n",
    )]);
    let results = EvalsGuidelinesAdoption.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Advisory);
    assert!(results[0].message.contains("docs/how-to/evals.md"));
    assert!(results[0].fix.is_some());
}

#[test]
fn test_evals_adoption_advisory_when_no_eval_docs() {
    let root = write_repo(&[("README.md", "# my-tool\n")]);
    let results = EvalsGuidelinesAdoption.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Advisory);
    assert!(results[0].message.contains("no eval docs"));
}

#[test]
fn test_evals_adoption_ignores_non_eval_docs() {
    let root = write_repo(&[("docs/how-to/fixture.md", "# Fixture\n")]);
    let results = EvalsGuidelinesAdoption.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert!(results[0].message.contains("no eval docs"));
}

// ── ManagedBlockDrift ─────────────────────────────────────────────

use crate::managed_block::{BlockInjector, BlockRegistry};

fn drift_target(rel: &str, expected: &str, fix: &str) -> DriftTarget {
    DriftTarget::new(
        BlockDef::new("WAI"),
        PathBuf::from(rel),
        expected.to_string(),
        fix.to_string(),
    )
}

/// A file whose block carries a footer with a stale (wrong) sha.
fn drifted_footer_file(expected: &str) -> String {
    format!(
        "# head\n\n{}stale hand-edited body\n<!-- provenance: generator=genesis version=0.1.0 source=WAI sha={} -->\n{}",
        BlockDef::new("WAI").start_marker,
        content_sha8(expected),
        BlockDef::new("WAI").end_marker,
    )
}

#[test]
fn test_drift_reported_with_file_block_and_fix() {
    let expected = "# generated\ncontent";
    let root = write_repo(&[("AGENTS.md", &drifted_footer_file(expected))]);
    let check = ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
    let results = check.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Warning);
    assert!(
        results[0].message.contains("AGENTS.md"),
        "names file: {}",
        results[0].message
    );
    assert!(
        results[0].message.contains("WAI"),
        "names block: {}",
        results[0].message
    );
    assert_eq!(results[0].fix.as_deref(), Some("my-tool init"));
}

#[test]
fn test_current_block_no_finding() {
    let expected = "# generated\ncontent";
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    let injector = BlockInjector::new(reg).with_provenance("genesis");
    let root = write_repo(&[("AGENTS.md", "# head\n")]);
    injector
        .inject(&root.join("AGENTS.md"), "WAI", expected)
        .require("value");
    let check = ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
    assert!(check.run(&root).require("value").is_empty());
}

#[test]
fn test_footerless_drift_full_text_fallback() {
    // Older blocks have no footer — drift is detected by full-text compare.
    let block = BlockDef::new("WAI");
    let file = format!("{}old body{}", block.start_marker, block.end_marker);
    let root = write_repo(&[("AGENTS.md", &file)]);
    let check = ManagedBlockDrift::new(vec![drift_target("AGENTS.md", "new body", "my-tool init")]);
    let results = check.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Warning);

    // Matching content (no footer) is clean.
    let file = format!("{}new body{}", block.start_marker, block.end_marker);
    let root = write_repo(&[("AGENTS.md", &file)]);
    assert!(check.run(&root).require("value").is_empty());
}

#[test]
fn test_hand_edited_block_is_finding_not_crash() {
    let expected = "# generated\ncontent";
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    let injector = BlockInjector::new(reg).with_provenance("genesis");
    let root = write_repo(&[("AGENTS.md", "# head\n")]);
    injector
        .inject(&root.join("AGENTS.md"), "WAI", expected)
        .require("value");
    // Hand-edit: markers intact, body changed, footer untouched.
    let edited = std::fs::read_to_string(root.join("AGENTS.md"))
        .require("value")
        .replace("content", "hand-edited");
    std::fs::write(root.join("AGENTS.md"), edited).require("value");
    let check = ManagedBlockDrift::new(vec![drift_target("AGENTS.md", expected, "my-tool init")]);
    let results = check.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Warning);
}

#[test]
fn test_missing_block_is_advisory() {
    let root = write_repo(&[("AGENTS.md", "# head — no block here\n")]);
    let check = ManagedBlockDrift::new(vec![drift_target("AGENTS.md", "anything", "my-tool init")]);
    let results = check.run(&root).require("value");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Advisory);
}

#[test]
fn test_managed_block_drift_registry_wiring() {
    let expected = "# generated";
    let root = write_repo(&[("AGENTS.md", &drifted_footer_file(expected))]);
    let mut registry = LinterRegistry::new();
    registry.register(Box::new(ManagedBlockDrift::new(vec![drift_target(
        "AGENTS.md",
        expected,
        "my-tool init",
    )])));
    let results = registry
        .run_named("genesis.managed_block_drift", &root)
        .require("check registered");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].severity, Severity::Warning);
}
