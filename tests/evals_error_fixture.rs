//! RED tests for genesis-built error-envelope fixtures.
//!
//! Task 2.1 of `fix-envelope-hint-channel` (beads genesis-jsu).
//!
//! Spec delta: `openspec/changes/fix-envelope-hint-channel/specs/evals/spec.md`
//! — "Eval fixtures use genesis-built envelopes": a fixture built with
//! `Envelope::error` carrying an `ErrorResult` with one remediation entry
//! SHALL resolve to a non-empty hint-command list via `parse_envelope`.
//!
//! RED today: `Envelope::error` emits remediation under `data.remediation`
//! but sets top-level `hints` to `None`, and `parse_envelope` only reads the
//! top-level `hints` channel — so the command the writer emitted is
//! invisible to the reader.

use genesis::envelope::{Envelope, EnvelopeKind, ErrorResult, RemediationEntry, UnmetClause};
use genesis::evals::{EnvelopeOutcome, parse_envelope};

/// One remediation entry naming a fix command.
fn remediation(command: &str) -> Vec<RemediationEntry> {
    vec![RemediationEntry {
        command: command.to_string(),
        description: "fix it".to_string(),
    }]
}

/// An `ErrorResult` carrying one remediation entry.
fn error_result_with_remediation(command: &str) -> ErrorResult {
    ErrorResult::new(
        "ERR_TEST",
        "something broke",
        None,
        None,
        None,
        Vec::<UnmetClause>::new(),
        remediation(command),
    )
    .expect("remediation is non-empty")
}

/// Scenario: a genesis-built error envelope resolves its remediation.
///
/// RED today: the reader cannot see the `data.remediation` channel.
#[test]
fn genesis_built_error_envelope_resolves_remediation() {
    let envelope = Envelope::error(
        "0.1",
        "my-tool/1.0.0",
        error_result_with_remediation("my-tool fix"),
        Vec::new(),
    );
    let stdout = serde_json::to_string(&envelope).expect("serialize envelope");
    let outcome = parse_envelope(&stdout).expect("parse");
    match &outcome {
        EnvelopeOutcome::Error { hint_commands, .. } => {
            assert!(
                !hint_commands.is_empty(),
                "a genesis-built error envelope with remediation must resolve a non-empty \
                 hint-command list; serialized: {stdout}"
            );
            assert!(
                hint_commands.contains(&"my-tool fix".to_string()),
                "the remediation command 'my-tool fix' must be among the hint commands; \
                 got {hint_commands:?} (serialized: {stdout})"
            );
        }
        other => panic!("expected an Error outcome, got {other:?}"),
    }
}

/// Scenario: a genesis-built success envelope parses as ok.
///
/// Green today: the success path is unaffected by the change.
#[test]
fn genesis_built_success_envelope_parses_as_ok() {
    let envelope = Envelope::success(
        "0.1",
        "my-tool/1.0.0",
        EnvelopeKind::Ok,
        serde_json::json!({}),
        Vec::new(),
        Vec::new(),
    );
    let stdout = serde_json::to_string(&envelope).expect("serialize envelope");
    let outcome = parse_envelope(&stdout).expect("parse");
    assert_eq!(outcome, EnvelopeOutcome::Ok);
}

/// Writer-shape guard: the remediation entry lands under `/data/remediation`.
///
/// This is green today (that is the writer shape); it exists so that a
/// future writer regression — e.g. moving remediation to the top level —
/// fails here rather than silently passing the reader tests.
#[test]
fn genesis_error_fixture_carries_remediation_under_data_remediation() {
    let envelope = Envelope::error(
        "0.1",
        "my-tool/1.0.0",
        error_result_with_remediation("my-tool fix"),
        Vec::new(),
    );
    let stdout = serde_json::to_string(&envelope).expect("serialize envelope");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON");
    let entry = value
        .pointer("/data/remediation/0")
        .expect("remediation must be serialized under data.remediation");
    assert_eq!(
        entry.get("command").and_then(|c| c.as_str()),
        Some("my-tool fix"),
        "the entry must carry its command under data.remediation[0].command"
    );
}
