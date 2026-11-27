//! Receipt metadata tests: terminal outcome serialization, round-trips,
//! and receipt-carrying envelopes (add-aix-eval-loop §1).

// pretender: role=test
use super::*;

// ── Receipt metadata (add-aix-eval-loop §1) ─────────────────────

#[test]
fn test_terminal_outcome_variants_serialize_snake_case() {
    let cases = vec![
        (TerminalOutcome::Success, "success"),
        (TerminalOutcome::Failure, "failure"),
        (TerminalOutcome::Timeout, "timeout"),
        (TerminalOutcome::Cancelled, "cancelled"),
    ];

    for (outcome, expected) in cases {
        let json = serde_json::to_string(&outcome).require("value");
        assert_eq!(json, format!("\"{}\"", expected), "variant {:?}", outcome);
        let back: TerminalOutcome = serde_json::from_str(&json).require("value");
        assert_eq!(back, outcome, "round-trip of {:?}", outcome);
    }
}

#[test]
fn test_receipt_meta_round_trips() {
    let receipt = ReceiptMeta {
        terminal_outcome: TerminalOutcome::Timeout,
        attempt: 3,
        idempotency_key: Some("deploy-config-v2".into()),
        evidence: Some("file exists at /tmp/out.txt".into()),
    };

    let json = serde_json::to_string(&receipt).require("value");
    let back: ReceiptMeta = serde_json::from_str(&json).require("value");
    assert_eq!(back, receipt);
}

/// Exhaustive over the receipt field space: every terminal outcome ×
/// optionality of `idempotency_key` and `evidence`. Stands in for a
/// property test without adding a proptest dev-dependency.
#[test]
fn test_receipt_round_trip_preserves_all_fields() {
    let outcomes = [
        TerminalOutcome::Success,
        TerminalOutcome::Failure,
        TerminalOutcome::Timeout,
        TerminalOutcome::Cancelled,
    ];
    let optionals = [
        (None, None),
        (Some("idem-key".to_string()), None),
        (None, Some("verifiable statement".to_string())),
        (
            Some("idem-key".to_string()),
            Some("verifiable statement".to_string()),
        ),
    ];
    for outcome in outcomes {
        for (key, evidence) in optionals.iter() {
            let receipt = ReceiptMeta {
                terminal_outcome: outcome,
                attempt: 7,
                idempotency_key: key.clone(),
                evidence: evidence.clone(),
            };
            let json = serde_json::to_string(&receipt).require("serialize receipt");
            let back: ReceiptMeta = serde_json::from_str(&json).require("deserialize receipt");
            assert_eq!(back, receipt, "round-trip with {outcome:?}");
        }
    }
}

#[test]
fn test_envelope_without_receipt_has_no_receipt_key() {
    let env = Envelope::success(
        "0.1",
        "my-tool/1.0.0",
        EnvelopeKind::Ok,
        "hello",
        vec![],
        vec![],
    );
    let json = serde_json::to_string(&env).require("value");
    let parsed: serde_json::Value = serde_json::from_str(&json).require("value");
    assert!(parsed.get("receipt").is_none());
}

/// Proves `parse_envelope` stays lenient: an enriched envelope (receipt set)
/// must parse with no change to its return type.
#[test]
fn test_with_receipt_builder_round_trips_through_parse_envelope() {
    let receipt = ReceiptMeta {
        terminal_outcome: TerminalOutcome::Failure,
        attempt: 1,
        idempotency_key: Some("mutating-cmd".into()),
        evidence: Some("no file was written".into()),
    };
    let env = Envelope::success(
        "0.1",
        "my-tool/1.0.0",
        EnvelopeKind::Ok,
        "hello",
        vec![],
        vec![],
    )
    .with_receipt(receipt);
    assert!(env.receipt.is_some());

    let json = serde_json::to_string(&env).require("value");
    let outcome = crate::evals::parse_envelope(&json).require("value");
    assert_eq!(outcome, crate::evals::EnvelopeOutcome::Ok);

    // Full serde round-trip preserves the receipt.
    let back: Envelope<String> = serde_json::from_str(&json).require("value");
    assert_eq!(
        back.receipt.as_ref().map(|r| r.terminal_outcome),
        Some(TerminalOutcome::Failure)
    );
    assert_eq!(back.receipt.as_ref().map(|r| r.attempt), Some(1));
}

#[test]
fn test_receipt_optional_fields_omitted_when_none() {
    let receipt = ReceiptMeta {
        terminal_outcome: TerminalOutcome::Success,
        attempt: 1,
        idempotency_key: None,
        evidence: None,
    };
    let json = serde_json::to_string(&receipt).require("value");
    let parsed: serde_json::Value = serde_json::from_str(&json).require("value");
    assert!(parsed.get("idempotency_key").is_none());
    assert!(parsed.get("evidence").is_none());
}
