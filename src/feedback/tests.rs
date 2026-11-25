//! Tests for the feedback entry point and title/body partitioning.

use super::content::{apply_title_override, stdin_title_and_body};
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
    let record = ErrorRecord {
        ts: "2026-07-29T12:00:00Z".into(),
        argv: vec![tool.to_string(), "check".into()],
        exit: exit_code,
        footer: Some("run doctor".into()),
        kind: "error".into(),
    };
    scratch::write_scratch_best_effort(tool, &record);
}

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

// --- stdin title/body partitioning (genesis-gle, genesis-og6) ---

#[test]
fn stdin_single_line_keeps_generic_title() {
    let (title, body) = stdin_title_and_body("bug", "summary line");
    assert_eq!(title, "[bug] feedback report");
    assert!(body.contains("summary line"), "body must carry the input");
}

#[test]
fn stdin_trailing_newline_is_still_single_line() {
    // `echo "x" | tool feedback bug` produces "x\n" — must keep the
    // generic title, not treat the trailing newline as a second line.
    let (title, body) = stdin_title_and_body("bug", "summary line\n");
    assert_eq!(title, "[bug] feedback report");
    assert!(body.contains("summary line"));
}

#[test]
fn stdin_multiline_promotes_first_line_to_title() {
    let input = "Bare row-id references fail when intent id has a dot\n\nThe resolution algorithm is undocumented. Details follow.\n";
    let (title, body) = stdin_title_and_body("bug", input);
    assert_eq!(
        title,
        "[bug] Bare row-id references fail when intent id has a dot"
    );
    assert!(body.contains("The resolution algorithm is undocumented."));
    assert!(
        !body.contains("Bare row-id references"),
        "first line must not be duplicated into the body"
    );
    assert!(body.starts_with("## Description"));
}

#[test]
fn stdin_multiline_blank_lines_around_single_line_is_still_single() {
    // A title line followed by blank lines trims to one line — the
    // echo-compat rule wins over title promotion.
    let (title, body) = stdin_title_and_body("feature", "add a flag\n\n");
    assert_eq!(title, "[feature] feedback report");
    assert!(body.contains("add a flag"));
}

#[test]
fn stdin_multiline_title_plus_body_omits_first_line_from_body() {
    let (title, body) = stdin_title_and_body("feature", "add a flag\n\nbody text\n");
    assert_eq!(title, "[feature] add a flag");
    assert!(body.contains("body text"));
    assert!(!body.contains("add a flag"));
}

#[test]
fn title_override_wins_over_derived_and_first_line() {
    assert_eq!(
        apply_title_override("bug", "[bug] feedback report", Some("My title")),
        "[bug] My title"
    );
    assert_eq!(
        apply_title_override(
            "bug",
            "[bug] auto-reported error: ah check",
            Some("My title")
        ),
        "[bug] My title"
    );
    assert_eq!(
        apply_title_override("bug", "[bug] feedback report", None),
        "[bug] feedback report",
        "no override keeps the derived title"
    );
}

#[test]
fn with_title_builder_sets_override() {
    let args = FeedbackArgs::new("bug", true, false).with_title("My title");
    assert_eq!(args.title.as_deref(), Some("My title"));
    // ::new keeps the 3-arg signature — no override by default.
    let plain = FeedbackArgs::new("bug", true, false);
    assert_eq!(plain.title, None);
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
