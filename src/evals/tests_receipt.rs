//! Receipt-check tests (split from tests.rs for the pretender file-lines
//! budget).

use super::*;
use crate::envelope::TerminalOutcome;
use crate::testutil::TestUnwrap;

// -- receipt_records_terminal_outcome (add-artifact-provenance §4) ----

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

/// A one-step replay result over [`receipt_step`].
fn receipt_result(stdout: String) -> ScenarioResult {
    ScenarioResult {
        steps: receipt_step(stdout),
        fixture_root: std::path::PathBuf::from("/tmp/unused"),
        distractors: vec![],
    }
}

/// The tool-fault reason of `outcome`, panicking on any other outcome.
fn expect_tool_fault(outcome: CheckOutcome) -> String {
    match outcome {
        CheckOutcome::Fail {
            taxonomy: None,
            reason,
        } => reason,
        other => panic!("expected tool fault, got {other:?}"),
    }
}

/// The receipt check composed with `ok_envelope`, run on one step.
fn run_composed_scenario(name: &str, stdout: String) -> ScenarioReport {
    Scenario::new(name, "prompt")
        .check("ok-envelope", ok_envelope(0))
        .check(
            "receipt-records-outcome",
            receipt_records_terminal_outcome(0),
        )
        .run(receipt_step(stdout))
        .require("fixture")
}

#[test]
fn test_receipt_present_passes() {
    // RED 4.1a: envelope carries receipt.terminal_outcome → pass.
    let check = receipt_records_terminal_outcome(0);
    let result = receipt_result(envelope_with_receipt(true, Some(TerminalOutcome::Success)));
    assert_eq!(check(&result), CheckOutcome::Pass);
}

#[test]
fn test_receipt_failure_over_failing_envelope_passes() {
    // Honest failure receipt on ok:false is consistent — pass.
    let check = receipt_records_terminal_outcome(0);
    let result = receipt_result(envelope_with_receipt(false, Some(TerminalOutcome::Failure)));
    assert_eq!(check(&result), CheckOutcome::Pass);
}

#[test]
fn test_missing_receipt_is_tool_fault_citing_receipt() {
    // RED 4.1b: no receipt on a mutating step → tool fault naming it.
    let check = receipt_records_terminal_outcome(0);
    let result = receipt_result(envelope_with_receipt(true, None));
    let reason = expect_tool_fault(check(&result));
    assert!(reason.contains("receipt"), "cites receipt: {reason}");
}

#[test]
fn test_success_receipt_over_failing_envelope_is_lie() {
    // RED 4.1c: ok:false + terminal_outcome=success → tool fault naming
    // the contradiction.
    let check = receipt_records_terminal_outcome(0);
    let result = receipt_result(envelope_with_receipt(false, Some(TerminalOutcome::Success)));
    let reason = expect_tool_fault(check(&result));
    assert!(
        reason.contains("ok:false") || reason.contains("ok: false"),
        "names contradiction: {reason}"
    );
    assert!(
        reason.contains("success"),
        "names the receipt claim: {reason}"
    );
}

#[test]
fn test_receipt_check_composes_into_scenario() {
    // RED 4.3: composes with existing checks without breaking them.
    let report = run_composed_scenario(
        "mutating-with-receipt",
        envelope_with_receipt(true, Some(TerminalOutcome::Success)),
    );
    assert!(report.passed, "failures: {:?}", report.failures);

    // Same scenario without a receipt: only the new check fails.
    let report = run_composed_scenario("mutating-no-receipt", envelope_with_receipt(true, None));
    assert!(!report.passed);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].0, "receipt-records-outcome");
}
