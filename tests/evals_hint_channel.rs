//! RED tests for dual-channel hint resolution in `parse_envelope`.
//!
//! Task 1.1 of `fix-envelope-hint-channel` (beads genesis-jsu).
//!
//! Spec delta: `openspec/changes/fix-envelope-hint-channel/specs/evals/spec.md`
//! — "Dual-channel hint resolution". The reader must resolve hint commands
//! from BOTH the `data.remediation` error channel and the legacy top-level
//! `hints` channel, in a stable union order (error channel first), and must
//! tolerate bare-string entries.
//!
//! These raw JSON literals stand in for captured tool stdout here; the
//! fixture-from-constructors requirement is exercised in
//! `tests/evals_error_fixture.rs` (task 2.1).

use genesis::evals::{EnvelopeOutcome, parse_envelope};

/// Scenario: remediation in the error channel is resolved.
///
/// RED today: the reader only looks at top-level `hints`.
#[test]
fn remediation_in_error_channel_is_resolved() {
    let stdout = r#"{
        "ok": false,
        "envelope_version": "0.1",
        "cli_version": "my-tool/1.0.0",
        "envelope_kind": "error",
        "data": {
            "code": "ERR_TEST",
            "message": "broken",
            "remediation": [
                {"command": "my-tool fix", "description": "fix it"}
            ]
        },
        "warnings": []
    }"#;
    let outcome = parse_envelope(stdout).expect("parse");
    assert_eq!(
        outcome,
        EnvelopeOutcome::Error {
            code: Some("ERR_TEST".to_string()),
            hint_commands: vec!["my-tool fix".to_string()],
        },
        "the data.remediation command must be resolved from the error channel"
    );
}

/// Scenario: top-level hints are resolved (legacy channel, already green).
#[test]
fn top_level_hints_are_resolved() {
    let stdout = r#"{
        "ok": false,
        "data": {
            "code": "ERR_TEST",
            "remediation": []
        },
        "hints": [{"command": "a", "description": "d"}]
    }"#;
    let outcome = parse_envelope(stdout).expect("parse");
    assert_eq!(
        outcome,
        EnvelopeOutcome::Error {
            code: Some("ERR_TEST".to_string()),
            hint_commands: vec!["a".to_string()],
        }
    );
}

/// Scenario: both channels contribute — union with the error channel first.
///
/// RED today: the error-channel command is invisible to the reader.
#[test]
fn both_channels_union_with_error_channel_first() {
    let stdout = r#"{
        "ok": false,
        "data": {
            "code": "ERR_TEST",
            "remediation": [{"command": "my-tool fix", "description": "fix it"}]
        },
        "hints": [{"command": "a", "description": "d"}]
    }"#;
    let outcome = parse_envelope(stdout).expect("parse");
    assert_eq!(
        outcome,
        EnvelopeOutcome::Error {
            code: Some("ERR_TEST".to_string()),
            hint_commands: vec!["my-tool fix".to_string(), "a".to_string()],
        },
        "both channels contribute; the error-channel command comes first and neither is discarded"
    );
}

/// Scenario: string-valued hint entries are tolerated, in both channels.
///
/// RED today: bare strings are dropped by the object-only reader.
#[test]
fn bare_string_entries_are_tolerated_in_both_channels() {
    let stdout = r#"{
        "ok": false,
        "data": {
            "code": "ERR_TEST",
            "remediation": ["cmd-x"]
        },
        "hints": ["cmd-y"]
    }"#;
    let outcome = parse_envelope(stdout).expect("parse must not fail on string entries");
    assert_eq!(
        outcome,
        EnvelopeOutcome::Error {
            code: Some("ERR_TEST".to_string()),
            hint_commands: vec!["cmd-x".to_string(), "cmd-y".to_string()],
        }
    );
}

/// Scenario: an envelope with no hint channel resolves to no commands.
#[test]
fn envelope_with_neither_channel_yields_zero_commands() {
    let stdout = r#"{"ok": false, "data": {}}"#;
    let outcome = parse_envelope(stdout).expect("parse");
    assert_eq!(
        outcome,
        EnvelopeOutcome::Error {
            code: None,
            hint_commands: vec![],
        },
        "no channels means zero hint commands and no error"
    );
}

/// The success path is untouched by the change.
#[test]
fn success_path_still_parses_as_ok() {
    let outcome = parse_envelope(r#"{"ok": true}"#).expect("parse");
    assert_eq!(outcome, EnvelopeOutcome::Ok);
}
