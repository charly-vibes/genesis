//! Tests for the feedback entry point and title/body partitioning.

use super::content;
use super::*;
use crate::evals::Scenario;
use scratch::ErrorRecord;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

/// Unique tool name per call: label + pid + a per-process counter.
/// The scratch store is keyed by tool name (genesis-kpv: two calls
/// sharing a label raced on the same scratch dir when tests ran in
/// parallel), so uniqueness must be per call, not per label.
fn test_tool(label: &str) -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    format!(
        "test-{label}-{}-{}",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}

fn write_scratch(tool: &str, exit_code: i32) {
    let mut record = ErrorRecord::new(
        "2026-07-29T12:00:00Z",
        vec![tool.to_string(), "check".into()],
        exit_code,
        "error",
    );
    record.footer = Some("run doctor".into());
    record.message = Some(format!("{}: simulated failure", tool));
    scratch::write_scratch_best_effort(tool, &record);
}

#[path = "tests_message.rs"]
mod tests_message;

#[path = "tests_title_body.rs"]
mod tests_title_body;

#[test]
fn test_handle_feedback_dry_run() {
    let dir = tmp();
    let tool = test_tool("dry-run");
    write_scratch(&tool, 1);

    let args = FeedbackArgs::new("bug", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(result.is_ok(), "dry run should succeed: {:?}", result);
}

#[test]
fn test_handle_feedback_from_last_error_no_scratch() {
    let dir = tmp();
    let tool = test_tool("no-scratch");
    let args = FeedbackArgs::new("bug", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(result.is_err(), "should fail when no scratch exists");
    assert!(
        result.unwrap_err().contains("No recent error"),
        "should mention missing scratch"
    );
}

#[test]
fn test_handle_feedback_no_from_last_error() {
    let dir = tmp();
    let tool = test_tool("no-from-last");
    let args = FeedbackArgs::new("bug", true, false);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(result.is_err(), "should fail without --from-last-error");
    assert!(
        result.unwrap_err().contains("--from-last-error"),
        "should suggest --from-last-error"
    );
}

#[test]
fn test_handle_feedback_invalid_kind() {
    let dir = tmp();
    let tool = test_tool("invalid-kind");
    let args = FeedbackArgs::new("invalid-kind", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(result.is_err(), "should reject invalid kind");
    let err = result.unwrap_err();
    assert!(err.contains("unknown kind"), "should say 'unknown kind'");
}

#[test]
fn test_handle_feedback_typo_suggestion_for_kind() {
    let dir = tmp();
    let tool = test_tool("typo");
    let args = FeedbackArgs::new("featuer", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(result.is_err(), "should reject typo kind");
    let err = result.unwrap_err();
    // Should contain a 'Did you mean' suggestion
    assert!(
        err.contains("feature") || err.contains("Did you mean"),
        "typo should suggest correct kind: {}",
        err
    );
}

#[test]
fn test_valid_kinds_list_matches_validation() {
    for kind in VALID_KINDS {
        let args = FeedbackArgs::new(kind.to_string(), true, true);
        let dir = tmp();
        let tool = test_tool("valid-kind");
        write_scratch(&tool, 1);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(
            result.is_ok(),
            "kind '{}' should be valid: {:?}",
            kind,
            result
        );
    }
}

#[test]
fn test_dry_run_prints_to_stderr() {
    // Verify dry_run returns FallbackUrl (not actually creating issue)
    let dir = tmp();
    let tool = test_tool("dry-run-2");
    write_scratch(&tool, 1);
    let args = FeedbackArgs::new("bug", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    match result {
        Ok(gh::GhResult::FallbackUrl(url)) => {
            assert!(url.contains("github.com"), "URL should contain github.com");
        }
        Ok(other) => panic!("expected FallbackUrl, got {:?}", other),
        Err(e) => panic!("should not error: {}", e),
    }
}

// -- aix-gap kind (add-artifact-provenance §5) ------------------------

#[test]
fn test_aix_gap_kind_is_accepted() {
    let dir = tmp();
    let tool = test_tool("aix-gap");
    write_scratch(&tool, 1);
    let args = FeedbackArgs::new("aix-gap", true, true);
    let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
    assert!(
        result.is_ok(),
        "aix-gap should pass validation: {:?}",
        result
    );
}

#[test]
fn test_aix_gap_near_misses_suggest_aix_gap() {
    let dir = tmp();
    let tool = test_tool("aix-gap-typo");
    for kind in ["aix_gap", "aixgap"] {
        let args = FeedbackArgs::new(kind, true, false);
        let result = handle_feedback(&args, &tool, "0.1.0", "owner/repo", dir.path());
        assert!(result.is_err(), "'{kind}' should be rejected");
        let err = result.unwrap_err();
        assert!(
            err.contains("aix-gap"),
            "'{kind}' should suggest aix-gap: {err}"
        );
    }
}

#[test]
fn test_aix_gap_capture_converts_to_replayable_scenario() {
    // The bundle an AIX-artifact consumer captures on a stale-artifact
    // failure: sync exited 1 pointing at a repair command.
    let bundle = stale_artifact_bundle();
    let scenario = Scenario::from_feedback_context(&bundle, vec![]);
    let report = scenario.run(vec![stale_artifact_step()]).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

/// The AIX-artifact failure bundle: sync exited 1 with a repair hint.
fn stale_artifact_bundle() -> context::ContextBundle {
    context::ContextBundle {
        tool_name: "my-tool".into(),
        tool_version: "1.0.0".into(),
        command: Some("my-tool sync".into()),
        exit_code: Some(1),
        suggestion_footer: Some("llms.txt stale\n  → Run: my-tool sync --force".into()),
        os_arch: "linux/x86_64".into(),
        shell: None,
        gh_version: None,
        git_remote: None,
        git_branch: None,
        git_dirty: None,
        repo_state: vec![],
        repro_hash: 0xa17, // arbitrary stable signature
    }
}

/// The recorded transcript step: sync failed with the repair hint.
fn stale_artifact_step() -> crate::evals::AgentStep {
    crate::evals::AgentStep {
        command: "my-tool sync".into(),
        stdout: serde_json::json!({
            "ok": false,
            "data": {"code": "E_STALE"},
            "hints": [{"command": "my-tool sync --force"}]
        })
        .to_string(),
        stderr: String::new(),
        exit_code: 1,
        executed: true,
    }
}

// genesis-ase: the unified path must not drop command/exit/footer from the
// `## Environment` section (wai reference wiring, wai/src/commands/feedback.rs).

fn context_bundle_for_scratch(tool: &str, dir: &Path) -> String {
    let args = FeedbackArgs::new("bug", true, true);
    let issue = content::issue_title_and_body(&args, tool).expect("body builds from scratch");
    context_bundle_section(
        tool,
        "0.1.0",
        dir,
        issue.command,
        issue.exit_code,
        issue.suggestion_footer,
    )
}

#[test]
fn handle_feedback_body_threads_record_fields_into_environment() {
    let dir = tmp();
    let tool = test_tool("env-fields");
    write_scratch(&tool, 1);
    let env = context_bundle_for_scratch(&tool, dir.path());
    assert!(
        env.contains(&format!("command: {tool} check")),
        "env: {env}"
    );
    assert!(env.contains("exit_code: 1"), "env: {env}");
    assert!(env.contains("suggestion: run doctor"), "env: {env}");
    assert!(env.contains("repro_hash:"), "env: {env}");
}

#[test]
fn record_repro_hash_delegates_to_context_scheme() {
    let record = ErrorRecord::new(
        "2026-07-29T12:00:00Z",
        vec!["tool".into(), "check".into()],
        1,
        "error",
    );
    let via_delegate = scenario::record_repro_hash(&record);
    let via_context = context::repro_hash("error", "2026-07-29T12:00:00Z", &None, None);
    assert_eq!(via_delegate, via_context);
    // Distinct records (different ts) hash differently — dedup still works.
    let other = ErrorRecord {
        ts: "2026-07-29T13:00:00Z".into(),
        ..record.clone()
    };
    assert_ne!(scenario::record_repro_hash(&other), via_delegate);
}
