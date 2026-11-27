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
            ..
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
            ..
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

// -- genesis-la4/0hx acceptance -------------------------------------------

/// Preamble-bearing stdout parses via the moved (now preamble-tolerant)
/// lenient reader — the property genesis-0zh's delegation relies on.
#[test]
fn test_parse_envelope_tolerates_preamble() {
    let out = parse_envelope("Using default project name: wai\n{\"ok\": false, \"data\": {}}")
        .require("value");
    assert!(
        matches!(out, EnvelopeOutcome::Error { .. }),
        "preamble before the envelope must not block parsing"
    );
}

/// `parse_envelope_value` consumes a `Value` with no string round-trip.
#[test]
fn test_parse_envelope_value_parses_without_round_trip() {
    let value: serde_json::Value = serde_json::from_str(
        r#"{"ok": false, "data": {"code": "E_CHECK"}, "hints": [{"command": "my-tool fix"}]}"#,
    )
    .require("value");
    let out = parse_envelope_value(&value).require("value");
    match out {
        EnvelopeOutcome::Error {
            code,
            hint_commands,
            ..
        } => {
            assert_eq!(code.as_deref(), Some("E_CHECK"));
            assert_eq!(hint_commands, vec!["my-tool fix".to_string()]);
        }
        other => panic!("expected error envelope, got {other:?}"),
    }
}

/// Malformed hint entries (naming no command) are counted in the outcome,
/// not dropped silently.
#[test]
fn test_parse_envelope_counts_malformed_hints() {
    let out = parse_envelope(
        r#"{"ok": false, "data": {"code": "E_M",
                "remediation": [{"command": "a fix"}, {"description": "no command"}]}}"#,
    )
    .require("value");
    match out {
        EnvelopeOutcome::Error {
            hint_commands,
            malformed_hints,
            ..
        } => {
            assert_eq!(hint_commands, vec!["a fix".to_string()]);
            assert_eq!(malformed_hints, 1);
        }
        other => panic!("expected error envelope, got {other:?}"),
    }
}
