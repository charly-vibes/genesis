//! Drift-check and evals-adoption filesystem tests (split from tests.rs for
//! the pretender file-lines and lazy-cluster budget).

use std::path::{Path, PathBuf};

use super::*;
use crate::managed_block::{BlockDef, BlockInjector, BlockRegistry, content_sha8};

use crate::testutil::TestUnwrap;

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

/// Inject a provenance-stamped `WAI` block holding `content` into
/// `root/AGENTS.md` via the real injector.
fn inject_block(root: &Path, content: &str) {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    let injector = BlockInjector::new(reg).with_provenance("genesis");
    injector
        .inject(&root.join("AGENTS.md"), "WAI", content)
        .require("value");
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
    let root = write_repo(&[("AGENTS.md", "# head\n")]);
    inject_block(&root, expected);
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
    let root = write_repo(&[("AGENTS.md", "# head\n")]);
    inject_block(&root, expected);
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
