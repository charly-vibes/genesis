//! Unit tests for `evals` (extracted from the inline `mod tests`
//! to keep the app file under the pretender file-lines budget).

// pretender: role=test
use crate::testutil::TestUnwrap;

use super::*;

// -- Taxonomy -----------------------------------------------------------

#[test]
fn test_taxonomy_codes_round_trip() {
    let all = [
        ErrorTaxonomy::EnvelopeHintBlindness,
        ErrorTaxonomy::StateMachineViolation,
        ErrorTaxonomy::ManagedBlockCorruption,
        ErrorTaxonomy::ToolExecutionHallucination,
        ErrorTaxonomy::ContextRecoveryFailure,
        ErrorTaxonomy::ToolDiscoveryFailure,
        ErrorTaxonomy::DocDriftBlindness,
        ErrorTaxonomy::ActionFormatViolation,
    ];
    for t in all {
        assert_eq!(ErrorTaxonomy::from_code(t.code()), Some(t));
    }
}

#[test]
fn test_action_format_violation_code() {
    // add-evals-guidelines: distinct from ToolExecutionHallucination
    // (a replay-tier code for never-executed claims).
    assert_eq!(
        ErrorTaxonomy::from_code("ERR_ACTION_FORMAT_VIOLATION"),
        Some(ErrorTaxonomy::ActionFormatViolation)
    );
    assert_eq!(
        ErrorTaxonomy::ActionFormatViolation.code(),
        "ERR_ACTION_FORMAT_VIOLATION"
    );
}

#[test]
fn test_preexisting_codes_unchanged() {
    // Guard: adding ActionFormatViolation must not move any existing code.
    assert_eq!(
        ErrorTaxonomy::EnvelopeHintBlindness.code(),
        "ERR_ENVELOPE_HINT_BLINDNESS"
    );
    assert_eq!(
        ErrorTaxonomy::StateMachineViolation.code(),
        "ERR_STATE_MACHINE_VIOLATION"
    );
    assert_eq!(
        ErrorTaxonomy::ManagedBlockCorruption.code(),
        "ERR_MANAGED_BLOCK_CORRUPTION"
    );
    assert_eq!(
        ErrorTaxonomy::ToolExecutionHallucination.code(),
        "ERR_TOOL_EXECUTION_HALLUCINATION"
    );
    assert_eq!(
        ErrorTaxonomy::ContextRecoveryFailure.code(),
        "ERR_CONTEXT_RECOVERY_FAILURE"
    );
    assert_eq!(
        ErrorTaxonomy::ToolDiscoveryFailure.code(),
        "ERR_TOOL_DISCOVERY_FAILURE"
    );
    assert_eq!(
        ErrorTaxonomy::DocDriftBlindness.code(),
        "ERR_DOC_DRIFT_BLINDNESS"
    );
}

#[test]
fn test_taxonomy_unknown_code_is_none() {
    assert_eq!(ErrorTaxonomy::from_code("ERR_NOPE"), None);
    assert_eq!(ErrorTaxonomy::from_code(""), None);
}

#[test]
fn test_taxonomy_display_contains_code() {
    let text = ErrorTaxonomy::ManagedBlockCorruption.to_string();
    assert!(text.starts_with("ERR_MANAGED_BLOCK_CORRUPTION"));
}

// -- Envelope parsing ----------------------------------------------------

#[test]
fn test_parse_envelope_ok() {
    let out = parse_envelope(r#"{"ok": true}"#).require("value");
    assert_eq!(out, EnvelopeOutcome::Ok);
}

#[test]
fn test_parse_envelope_error_with_hints() {
    let out = parse_envelope(
        r#"{"ok": false, "data": {"code": "E_CHECK"},
                "hints": [{"command": "my-tool fix", "description": "run fix"}]}"#,
    )
    .require("value");
    match out {
        EnvelopeOutcome::Error {
            code,
            hint_commands,
        } => {
            assert_eq!(code.as_deref(), Some("E_CHECK"));
            assert_eq!(hint_commands, vec!["my-tool fix".to_string()]);
        }
        other => panic!("expected error envelope, got {other:?}"),
    }
}

#[test]
fn test_parse_envelope_error_without_hints() {
    let out = parse_envelope(r#"{"ok": false, "data": {}}"#).require("value");
    match out {
        EnvelopeOutcome::Error {
            code,
            hint_commands,
        } => {
            assert_eq!(code, None);
            assert!(hint_commands.is_empty());
        }
        other => panic!("expected error envelope, got {other:?}"),
    }
}

#[test]
fn test_parse_envelope_rejects_non_json() {
    let err = parse_envelope("panic: boom").unwrap_err();
    assert!(matches!(err, EvalsError::NotJson(_)));
}

#[test]
fn test_parse_envelope_rejects_missing_ok() {
    let err = parse_envelope(r#"{"status": "fine"}"#).unwrap_err();
    assert!(matches!(err, EvalsError::NoOkField));
}

// -- End-to-end scenario --------------------------------------------------

const MANAGED_BLOCK: &str = "<!-- my-tool:START -->\nmanaged\n<!-- my-tool:END -->\n";

fn error_envelope_stdout(suggested: &str) -> String {
    serde_json::json!({
        "ok": false,
        "data": {"code": "E_STATE", "message": "bad state"},
        "hints": [{"command": suggested, "description": "suggested fix"}]
    })
    .to_string()
}

fn ok_envelope_stdout() -> String {
    serde_json::json!({"ok": true}).to_string()
}

/// Worked end-to-end scenario: fixture + fake-agent replay +
/// deterministic checks + taxonomy classification. No live LLM, no
/// subprocess runner — the transcript is replayed in memory.
#[test]
fn scenario_error_hint_followed_end_to_end() {
    let scenario = Scenario::new("hint-followed", "Run my-tool check and fix what breaks")
        .fixture_file("AGENTS.md", MANAGED_BLOCK)
        .check(
            "step0-error-envelope-with-hint",
            error_envelope_with_hint(0, "my-tool fix"),
        )
        .check("step0-nonzero-exit", |r| {
            // Portable assertion: nonzero + parseable envelope (holds
            // before/after genesis-u40 exit-code refinement).
            if r.steps[0].exit_code != 0 {
                CheckOutcome::pass()
            } else {
                CheckOutcome::tool_fault("step 0 exited 0")
            }
        })
        .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"))
        .check("recovery-ok-envelope", ok_envelope(1))
        .check("agent-executed-all", agent_executed_all());

    let replay = vec![
        AgentStep {
            command: "my-tool check".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
        AgentStep {
            command: "my-tool fix".into(),
            stdout: ok_envelope_stdout(),
            stderr: String::new(),
            exit_code: 0,
            executed: true,
        },
    ];

    let report = scenario.run(replay).require("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
    assert!(report.failures.is_empty());
    // Fixture materialized for checks.
    assert!(report.fixture_root.join("AGENTS.md").exists());
}

/// The same scenario against a hint-blind transcript classifies the
/// failure as ERR_ENVELOPE_HINT_BLINDNESS.
#[test]
fn scenario_classifies_hint_blindness() {
    let scenario = Scenario::new("hint-blind", "prompt")
        .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"));
    let replay = vec![
        AgentStep {
            command: "my-tool check".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
        AgentStep {
            command: "rm -rf /".into(),
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    assert_eq!(report.failures.len(), 1);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail {
            taxonomy: Some(t), ..
        } => assert_eq!(*t, ErrorTaxonomy::EnvelopeHintBlindness),
        other => panic!("expected agent fault, got {other:?}"),
    }
}

/// A recorded agent that followed the hint but appended a flag —
/// `my-tool fix --json` vs the suggested `my-tool fix` — followed the
/// hint. Exact-or-prefix matching (mirrors doc_drift_blindness):
/// flags/args must not classify hint-following as
/// ERR_ENVELOPE_HINT_BLINDNESS (evallerina-2rr).
#[test]
fn scenario_hint_followed_with_appended_flag_passes() {
    let scenario = Scenario::new("hint-with-flag", "prompt")
        .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"));
    let replay = vec![
        AgentStep {
            command: "my-tool check".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
        AgentStep {
            command: "my-tool fix --json".into(),
            stdout: ok_envelope_stdout(),
            stderr: String::new(),
            exit_code: 0,
            executed: true,
        },
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
}

/// A truly different command still classifies as
/// ERR_ENVELOPE_HINT_BLINDNESS — the prefix must be the suggested
/// command followed by a word boundary, not any prefix characters.
#[test]
fn scenario_different_command_still_hint_blind() {
    let scenario = Scenario::new("hint-different", "prompt")
        .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"));
    let replay = vec![
        AgentStep {
            command: "my-tool check".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
        AgentStep {
            // Shares a character prefix with the suggestion but is a
            // different command ("fixtur" vs "fix") — must stay blind.
            command: "my-tool fixtur".into(),
            stdout: ok_envelope_stdout(),
            stderr: String::new(),
            exit_code: 0,
            executed: true,
        },
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail {
            taxonomy: Some(t), ..
        } => assert_eq!(*t, ErrorTaxonomy::EnvelopeHintBlindness),
        other => panic!("expected agent fault, got {other:?}"),
    }
}

/// A step with `executed: false` classifies as
/// ERR_TOOL_EXECUTION_HALLUCINATION.
#[test]
fn scenario_classifies_hallucination() {
    let scenario =
        Scenario::new("hallucinated", "prompt").check("agent-executed-all", agent_executed_all());
    let replay = vec![AgentStep {
        command: "my-tool check".into(),
        stdout: ok_envelope_stdout(),
        stderr: String::new(),
        exit_code: 0,
        executed: false,
    }];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail {
            taxonomy: Some(t), ..
        } => assert_eq!(*t, ErrorTaxonomy::ToolExecutionHallucination),
        other => panic!("expected agent fault, got {other:?}"),
    }
}

/// A failing recovery envelope is a TOOL fault (fix issued but
/// ineffective), not ERR_ENVELOPE_HINT_BLINDNESS.
#[test]
fn ok_envelope_failing_recovery_is_tool_fault_not_blindness() {
    let scenario =
        Scenario::new("ineffective-fix", "prompt").check("recovery-ok-envelope", ok_envelope(1));
    let replay = vec![
        AgentStep {
            command: "my-tool check".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
        AgentStep {
            command: "my-tool fix".into(),
            stdout: error_envelope_stdout("my-tool fix"),
            stderr: String::new(),
            exit_code: 1,
            executed: true,
        },
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy: None, .. } => {}
        other => panic!("expected tool fault, got {other:?}"),
    }
}

/// Step-indexed checks against an empty replay report tool faults,
/// not panics.
#[test]
fn empty_replay_yields_tool_faults_for_step_checks() {
    let scenario = Scenario::new("empty", "prompt")
        .check(
            "step0-hint-envelope",
            error_envelope_with_hint(0, "my-tool fix"),
        )
        .check("step0-ok-envelope", ok_envelope(0));
    let report = scenario.run(vec![]).require("fixture");
    assert!(!report.passed);
    assert_eq!(report.failures.len(), 2);
    for (_, outcome) in &report.failures {
        match outcome {
            CheckOutcome::Fail { taxonomy: None, .. } => {}
            other => panic!("expected tool fault, got {other:?}"),
        }
    }
}

/// Tool-fault failures (hintless error envelope) carry no taxonomy.
#[test]
fn scenario_tool_fault_has_no_taxonomy() {
    let scenario = Scenario::new("hintless-tool", "prompt").check(
        "step0-error-envelope-with-hint",
        error_envelope_with_hint(0, "my-tool fix"),
    );
    let replay = vec![AgentStep {
        command: "my-tool check".into(),
        stdout: r#"{"ok": false, "data": {}}"#.into(), // error, no hints
        stderr: String::new(),
        exit_code: 1,
        executed: true,
    }];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy: None, .. } => {}
        other => panic!("expected tool fault, got {other:?}"),
    }
}
