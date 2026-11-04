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

// -- Report contract (add-evals-guidelines) ------------------------------

/// The non-tier-2 normative fixture must round-trip through
/// `ScenarioReport`'s serde serialization: its shared fields (name,
/// passed↔status, failures↔checks with taxonomy codes) match what the
/// replay-tier struct emits. Live-only fields (report_version, tool,
/// bounds, repetition) are documented as a delta in
/// docs/reference/eval-report.md.
#[test]
fn test_report_replay_fixture_round_trips_through_scenario_report() {
    let golden_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/eval_report_replay.json");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&golden_path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", golden_path.display())),
    )
    .expect("non-tier-2 fixture is valid JSON");

    assert_eq!(fixture["report_version"], 1);
    assert_eq!(fixture["status"], "passed");
    // Non-tier-2 rows omit tier-2 fields entirely (never null).
    assert!(fixture.get("model").is_none());
    assert!(fixture.get("repetition").is_none());

    // Rebuild the replay-tier report the fixture describes and serialize it.
    let scenario = Scenario::new(
        fixture["name"].as_str().require("value").to_owned(),
        "fixture round-trip",
    )
    .check("envelope_hint_followed", |_| CheckOutcome::pass());
    let report = scenario
        .run(vec![AgentStep {
            command: "my-tool status".into(),
            stdout: r#"{"ok": true}"#.into(),
            stderr: String::new(),
            exit_code: 0,
            executed: true,
        }])
        .expect("fixture materializes");
    let serialized: serde_json::Value =
        serde_json::to_value(&report).expect("ScenarioReport serializes");

    // Shared fields agree across the two shapes.
    assert_eq!(serialized["name"], fixture["name"]);
    assert_eq!(
        serialized["passed"].as_bool(),
        Some(fixture["status"].as_str() == Some("passed"))
    );
    assert_eq!(serialized.get("model"), None);
}

/// No field in either normative report fixture may serialize as `null` —
/// optional fields are omitted, never null (evals-guidelines).
#[test]
fn test_report_fixtures_contain_no_nulls() {
    for name in ["eval_report_tier2.json", "eval_report_replay.json"] {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/golden")
            .join(name);
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
        )
        .unwrap_or_else(|e| panic!("{name} is not valid JSON: {e}"));
        fn assert_no_nulls(v: &serde_json::Value, where_: &str) {
            match v {
                serde_json::Value::Null => panic!("null at {where_}"),
                serde_json::Value::Array(items) => {
                    for (i, item) in items.iter().enumerate() {
                        assert_no_nulls(item, &format!("{where_}[{i}]"));
                    }
                }
                serde_json::Value::Object(map) => {
                    for (k, val) in map {
                        assert_no_nulls(val, &format!("{where_}.{k}"));
                    }
                }
                _ => {}
            }
        }
        assert_no_nulls(&fixture, name);
    }
}

/// The tier-2 fixture demonstrates the fault-vocabulary rules: an
/// agent fault carries a code, a passing check and a tool fault
/// never do, and tier-2 rows carry model + repetition.
#[test]
fn test_report_tier2_fixture_vocabulary() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/eval_report_tier2.json");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display())),
    )
    .expect("tier-2 fixture is valid JSON");

    assert_eq!(fixture["report_version"], 1);
    assert_eq!(fixture["status"], "failed");
    // Tier-2 rows carry the raw model id and a 0-based repetition.
    assert_eq!(fixture["model"], "deepseek/deepseek-v4-flash:free");
    assert_eq!(fixture["repetition"], 0);
    // code present iff fault == agent.
    for check in fixture["checks"].as_array().require("value") {
        let has_code = check.get("code").is_some();
        let is_agent_fault = check["fault"].as_str() == Some("agent");
        assert_eq!(has_code, is_agent_fault, "check {}", check["name"]);
    }
    // Bounds recorded with values.
    assert!(fixture["bounds"]["max_turns"].is_u64());
    assert!(fixture["bounds"]["timeout_secs"].is_u64());
    assert!(fixture["bounds"]["token_budget"].is_u64());
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

    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
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
    let report = scenario.run(vec![]).expect("fixture");
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
    let report = scenario.run(replay).expect("fixture");
    assert!(!report.passed);
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy: None, .. } => {}
        other => panic!("expected tool fault, got {other:?}"),
    }
}

// -- receipt_records_terminal_outcome (add-artifact-provenance §4) ----

use crate::envelope::TerminalOutcome;

/// Envelope stdout with an optional receipt, for receipt-check tests.
fn envelope_with_receipt(ok: bool, outcome: Option<TerminalOutcome>) -> String {
    let mut json = serde_json::json!({"ok": ok});
    if let Some(outcome) = outcome {
        json["receipt"] = serde_json::json!({
            "terminal_outcome": outcome,
            "attempt": 1,
        });
    }
    json.to_string()
}

fn receipt_step(stdout: String) -> Vec<AgentStep> {
    vec![AgentStep {
        command: "my-tool mutate".into(),
        stdout,
        stderr: String::new(),
        exit_code: 0,
        executed: true,
    }]
}

#[test]
fn test_receipt_present_passes() {
    // RED 4.1a: envelope carries receipt.terminal_outcome → pass.
    let check = receipt_records_terminal_outcome(0);
    let result = ScenarioResult {
        steps: receipt_step(envelope_with_receipt(true, Some(TerminalOutcome::Success))),
        fixture_root: std::path::PathBuf::from("/tmp/unused"),
        distractors: vec![],
    };
    assert_eq!(check(&result), CheckOutcome::Pass);
}

#[test]
fn test_receipt_failure_over_failing_envelope_passes() {
    // Honest failure receipt on ok:false is consistent — pass.
    let check = receipt_records_terminal_outcome(0);
    let result = ScenarioResult {
        steps: receipt_step(envelope_with_receipt(false, Some(TerminalOutcome::Failure))),
        fixture_root: std::path::PathBuf::from("/tmp/unused"),
        distractors: vec![],
    };
    assert_eq!(check(&result), CheckOutcome::Pass);
}

#[test]
fn test_missing_receipt_is_tool_fault_citing_receipt() {
    // RED 4.1b: no receipt on a mutating step → tool fault naming it.
    let check = receipt_records_terminal_outcome(0);
    let result = ScenarioResult {
        steps: receipt_step(envelope_with_receipt(true, None)),
        fixture_root: std::path::PathBuf::from("/tmp/unused"),
        distractors: vec![],
    };
    match check(&result) {
        CheckOutcome::Fail {
            taxonomy: None,
            reason,
        } => {
            assert!(reason.contains("receipt"), "cites receipt: {reason}");
        }
        other => panic!("expected tool fault, got {other:?}"),
    }
}

#[test]
fn test_success_receipt_over_failing_envelope_is_lie() {
    // RED 4.1c: ok:false + terminal_outcome=success → tool fault naming
    // the contradiction.
    let check = receipt_records_terminal_outcome(0);
    let result = ScenarioResult {
        steps: receipt_step(envelope_with_receipt(false, Some(TerminalOutcome::Success))),
        fixture_root: std::path::PathBuf::from("/tmp/unused"),
        distractors: vec![],
    };
    match check(&result) {
        CheckOutcome::Fail {
            taxonomy: None,
            reason,
        } => {
            assert!(
                reason.contains("ok:false") || reason.contains("ok: false"),
                "names contradiction: {reason}"
            );
            assert!(
                reason.contains("success"),
                "names the receipt claim: {reason}"
            );
        }
        other => panic!("expected tool fault, got {other:?}"),
    }
}

#[test]
fn test_receipt_check_composes_into_scenario() {
    // RED 4.3: composes with existing checks without breaking them.
    let scenario = Scenario::new("mutating-with-receipt", "prompt")
        .check("ok-envelope", ok_envelope(0))
        .check(
            "receipt-records-outcome",
            receipt_records_terminal_outcome(0),
        );
    let replay = receipt_step(envelope_with_receipt(true, Some(TerminalOutcome::Success)));
    let report = scenario.run(replay).expect("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);

    // Same scenario without a receipt: only the new check fails.
    let scenario = Scenario::new("mutating-no-receipt", "prompt")
        .check("ok-envelope", ok_envelope(0))
        .check(
            "receipt-records-outcome",
            receipt_records_terminal_outcome(0),
        );
    let replay = receipt_step(envelope_with_receipt(true, None));
    let report = scenario.run(replay).expect("fixture");
    assert!(!report.passed);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].0, "receipt-records-outcome");
}
