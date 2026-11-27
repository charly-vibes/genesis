//! End-to-end scenario replay tests (split from tests.rs for the pretender
//! file-lines budget).

use super::*;
use crate::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry, UnmetClause};
use crate::testutil::TestUnwrap;

// -- End-to-end scenario --------------------------------------------------

const MANAGED_BLOCK: &str = "<!-- my-tool:START -->\nmanaged\n<!-- my-tool:END -->\n";

/// Genesis-built error envelope fixture (task 2.2): remediation lives in
/// the error channel `data.remediation`, top-level `hints` is `None`.
fn error_envelope_stdout(suggested: &str) -> String {
    let err = ErrorResult::new(
        "E_STATE",
        "bad state",
        None,
        None,
        None,
        Vec::<UnmetClause>::new(),
        vec![RemediationEntry {
            command: suggested.to_string(),
            description: "suggested fix".into(),
        }],
    )
    .expect("remediation is non-empty");
    serde_json::to_string(&Envelope::error("genesis/evals", err, Vec::new()))
        .expect("error envelope serializes")
}

fn ok_envelope_stdout() -> String {
    serde_json::to_string(&Envelope::success(
        "genesis/evals",
        EnvelopeKind::Ok,
        serde_json::json!({}),
        Vec::new(),
        Vec::new(),
    ))
    .expect("success envelope serializes")
}

/// AgentStep builder keeping replay tables one line per step.
fn step(command: &str, stdout: String, exit_code: i32) -> AgentStep {
    AgentStep {
        command: command.into(),
        stdout,
        stderr: String::new(),
        exit_code,
        executed: true,
    }
}

/// Assert the first failure carries the expected taxonomy.
fn assert_first_failure_taxonomy(report: &ScenarioReport, expected: Option<ErrorTaxonomy>) {
    let (_, outcome) = &report.failures[0];
    match outcome {
        CheckOutcome::Fail { taxonomy, .. } if *taxonomy == expected => {}
        other => panic!("expected taxonomy {expected:?}, got {other:?}"),
    }
}

/// Worked end-to-end scenario: fixture + fake-agent replay +
/// deterministic checks + taxonomy classification. No live LLM, no
/// subprocess runner — the transcript is replayed in memory.
#[test]
fn scenario_error_hint_followed_end_to_end() {
    let scenario = hint_followed_scenario();
    let replay = vec![
        step("my-tool check", error_envelope_stdout("my-tool fix"), 1),
        step("my-tool fix", ok_envelope_stdout(), 0),
    ];

    let report = scenario.run(replay).require("fixture");
    assert!(report.passed, "failures: {:?}", report.failures);
    assert!(report.failures.is_empty());
    // Fixture materialized for checks.
    assert!(report.fixture_root.join("AGENTS.md").exists());
}

/// The full hint-followed scenario: fixture + step-indexed checks.
fn hint_followed_scenario() -> Scenario {
    Scenario::new("hint-followed", "Run my-tool check and fix what breaks")
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
        .check("agent-executed-all", agent_executed_all())
}

/// The same scenario against a hint-blind transcript classifies the
/// failure as ERR_ENVELOPE_HINT_BLINDNESS.
#[test]
fn scenario_classifies_hint_blindness() {
    let scenario = Scenario::new("hint-blind", "prompt")
        .check("agent-followed-hint", agent_followed_hint(1, "my-tool fix"));
    let replay = vec![
        step("my-tool check", error_envelope_stdout("my-tool fix"), 1),
        step("rm -rf /", String::new(), 1),
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    assert_eq!(report.failures.len(), 1);
    assert_first_failure_taxonomy(&report, Some(ErrorTaxonomy::EnvelopeHintBlindness));
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
        step("my-tool check", error_envelope_stdout("my-tool fix"), 1),
        step("my-tool fix --json", ok_envelope_stdout(), 0),
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
        step("my-tool check", error_envelope_stdout("my-tool fix"), 1),
        // Shares a character prefix with the suggestion but is a
        // different command ("fixtur" vs "fix") — must stay blind.
        step("my-tool fixtur", ok_envelope_stdout(), 0),
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    assert_first_failure_taxonomy(&report, Some(ErrorTaxonomy::EnvelopeHintBlindness));
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
        step("my-tool check", error_envelope_stdout("my-tool fix"), 1),
        step("my-tool fix", error_envelope_stdout("my-tool fix"), 1),
    ];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    assert_first_failure_taxonomy(&report, None);
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
    let replay = vec![step(
        "my-tool check",
        r#"{"ok": false, "data": {}}"#.into(), // error, no hints,
        1,
    )];
    let report = scenario.run(replay).require("fixture");
    assert!(!report.passed);
    assert_first_failure_taxonomy(&report, None);
}
