//! Tests for the lenient reader moved into `envelope` (genesis-la4/0hx):
//! preamble tolerance, the Value-based entry point, and malformed-hint
//! counting.

// pretender: role=test
use crate::envelope::{EnvelopeOutcome, EnvelopeParseError, parse_envelope, parse_envelope_value};
use crate::testutil::TestUnwrap;
use serde_json::json;

// -- Preamble tolerance (genesis-la4) ---------------------------------

/// Tools may print human preamble (progress, notices) before the
/// envelope; the first `{` anywhere in stdout starts the payload.
#[test]
fn test_parse_envelope_tolerates_preamble() {
    let stdout = "Using default project name: wai\n\
                  {\"ok\": false, \"data\": {\"code\": \"E_X\"}, \
                  \"hints\": [\"wai fix\"]}";
    let outcome = parse_envelope(stdout).require("value");
    match outcome {
        EnvelopeOutcome::Error {
            code,
            hint_commands,
            malformed_hints,
        } => {
            assert_eq!(code.as_deref(), Some("E_X"));
            assert_eq!(hint_commands, vec!["wai fix".to_string()]);
            assert_eq!(malformed_hints, 0);
        }
        other => panic!("expected error outcome, got {other:?}"),
    }
}

#[test]
fn test_preamble_bearing_ok_envelope_parses() {
    let outcome = parse_envelope("notice: warming up\n{\"ok\": true}").require("value");
    assert_eq!(outcome, EnvelopeOutcome::Ok);
}

#[test]
fn test_stdout_without_any_json_is_not_json() {
    let err = parse_envelope("panic: boom").unwrap_err();
    assert!(matches!(err, EnvelopeParseError::NotJson(_)));
}

// -- Value-based entry point (genesis-0hx) ----------------------------

#[test]
fn test_parse_envelope_value_consumes_value_directly() {
    let value = json!({
        "ok": false,
        "data": {"code": "E_V"},
        "hints": [
            {"command": "v fix", "description": "object entry"},
            "bare fix",
            {"description": "no command — malformed"}
        ]
    });
    let outcome = parse_envelope_value(&value).require("value");
    match outcome {
        EnvelopeOutcome::Error {
            code,
            hint_commands,
            malformed_hints,
        } => {
            assert_eq!(code.as_deref(), Some("E_V"));
            assert_eq!(
                hint_commands,
                vec!["v fix".to_string(), "bare fix".to_string()]
            );
            assert_eq!(malformed_hints, 1);
        }
        other => panic!("expected error outcome, got {other:?}"),
    }
}

#[test]
fn test_parse_envelope_value_ok() {
    let outcome = parse_envelope_value(&json!({"ok": true})).require("value");
    assert_eq!(outcome, EnvelopeOutcome::Ok);
}

#[test]
fn test_parse_envelope_value_rejects_missing_ok() {
    let err = parse_envelope_value(&json!({"status": "fine"})).unwrap_err();
    assert!(matches!(err, EnvelopeParseError::NoOkField));
}

#[test]
fn test_parse_envelope_value_reads_both_hint_channels() {
    let value = json!({
        "ok": false,
        "data": {"code": "E_U", "remediation": [{"command": "channel one"}]},
        "hints": ["channel two"]
    });
    let outcome = parse_envelope_value(&value).require("value");
    match outcome {
        EnvelopeOutcome::Error {
            hint_commands,
            malformed_hints,
            ..
        } => {
            assert_eq!(
                hint_commands,
                vec!["channel one".to_string(), "channel two".to_string()]
            );
            assert_eq!(malformed_hints, 0);
        }
        other => panic!("expected error outcome, got {other:?}"),
    }
}
