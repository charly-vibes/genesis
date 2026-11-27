//! genesis-azy predictions, part 1 of 2: the envelope seam (P1). Each test
//! mirrors one scratchbook property; the pass/fail outcome is the
//! "predicted red/green" evidence recorded there.

mod common;

use genesis::envelope::{parse_envelope_value, Envelope, EnvelopeOutcome, ErrorResult, RemediationEntry};

/// A minimal ok:false envelope whose `data.remediation` is `remediation`.
fn envelope_json(remediation: &str) -> String {
    format!(
        r#"{{
        "ok": false,
        "envelope_version": "1.0.0",
        "cli_version": "t/0.1.0",
        "envelope_kind": "error",
        "data": {{
            "code": "E_X",
            "message": "m",
            "rule_name": null,
            "spec_ref": null,
            "entity_id": null,
            "unmet_clauses": [],
            "remediation": {remediation}
        }},
        "warnings": [],
        "hints": null,
        "meta": {{ "duration_ms": 0, "tx": null, "request_id": null }}
    }}"#
    )
}

#[test]
fn p1_error_envelope_remediation_visible() {
    let err = common::require(ErrorResult::new(
        "E_TEST",
        "something failed",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "genesis doctor --fix".into(),
            description: "run the fix".into(),
        }],
    ), "ErrorResult::new with remediation entry");
    let env = Envelope::error("1.0.0", "my-tool/0.1.0", err, vec![]);
    let json = common::require(serde_json::to_value(&env), "serialize envelope to JSON");
    match common::require(parse_envelope_value(&json), "parse error envelope") {
        EnvelopeOutcome::Error {
            code, hint_commands, ..
        } => {
            assert_eq!(code.as_deref(), Some("E_TEST"));
            assert!(
                hint_commands.iter().any(|c| c.contains("doctor --fix")),
                "remediation command invisible to parse_envelope_value: {hint_commands:?}"
            );
            println!("P1a RESULT: GREEN (constructor path exposes remediation)");
        }
        EnvelopeOutcome::Ok => panic!("error envelope parsed as Ok"),
    }
}

/// Derived Deserialize is a second construction path past Invariant 3.2.5.
#[test]
fn p1_deserialized_error_envelope_invariant() {
    let parsed: Result<Envelope<ErrorResult>, _> =
        serde_json::from_str(&envelope_json("[]"));
    match parsed {
        Ok(_) => panic!(
            "P1b RESULT: RED — Deserialize bypasses Invariant 3.2.5 (empty remediation accepted)"
        ),
        Err(_) => println!("P1b RESULT: GREEN (deserialize upholds Invariant 3.2.5)"),
    }
}

/// A deserialize-made error envelope with no fix anywhere: the reader
/// must report zero commands faithfully, never synthesize phantoms.
#[test]
fn p1_error_envelope_with_no_fix_anywhere_is_visible() {
    let raw = envelope_json("[]");
    let v: serde_json::Value = common::require(serde_json::from_str(&raw), "deserialize envelope json");
    match common::require(parse_envelope_value(&v), "parse zero-command error envelope") {
        EnvelopeOutcome::Error { hint_commands, .. } => {
            assert!(
                hint_commands.is_empty(),
                "phantom commands synthesized from nothing: {hint_commands:?}"
            );
            println!("P1c RESULT: GREEN (faithful zero-command report)");
        }
        EnvelopeOutcome::Ok => panic!("ok:false envelope parsed as Ok"),
    }
}
