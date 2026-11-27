//! Caller-supplied CLI version passthrough and parameterized
//! `envelope_version` tests (genesis-la4).

// pretender: role=test
use super::*;
use crate::testutil::TestUnwrap;

// ── Caller-supplied CLI version ──────────────────────────────────

#[test]
fn test_success_preserves_caller_cli_version() {
    let env = Envelope::success(
        "0.1",
        "downstream-tool/2.0.0",
        EnvelopeKind::Ok,
        "data",
        vec![],
        vec![],
    );
    assert_eq!(env.cli_version, "downstream-tool/2.0.0");
}

#[test]
fn test_error_preserves_caller_cli_version() {
    let err = ErrorResult::new(
        "E001",
        "msg",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "fix".into(),
            description: "run fix".into(),
        }],
    )
    .require("value");
    let env = Envelope::error("0.1", "downstream-tool/2.0.0", err, vec![]);
    assert_eq!(env.cli_version, "downstream-tool/2.0.0");
}

#[test]
fn test_success_with_tx_preserves_caller_cli_version() {
    let env = Envelope::success_with_tx(
        "0.1",
        "downstream/3.0.0",
        EnvelopeKind::Ok,
        (),
        vec![],
        vec![],
        Some(1),
    );
    assert_eq!(env.cli_version, "downstream/3.0.0");
}

// ── Envelope version parameterization (genesis-la4) ──────────────

/// The acceptance criterion for parameterized `envelope_version`: two
/// constructors given different per-tool versions produce different
/// `envelope_version` values — the field is the caller's, not a const.
#[test]
fn test_envelope_version_is_caller_supplied() {
    let tool_a = Envelope::success(
        "tool-a/0.1",
        "tool-a/1.0.0",
        EnvelopeKind::Ok,
        (),
        vec![],
        vec![],
    );
    let tool_b = Envelope::success(
        "tool-b/2.0",
        "tool-b/1.0.0",
        EnvelopeKind::Ok,
        (),
        vec![],
        vec![],
    );
    assert_eq!(tool_a.envelope_version, "tool-a/0.1");
    assert_eq!(tool_b.envelope_version, "tool-b/2.0");
    assert_ne!(tool_a.envelope_version, tool_b.envelope_version);

    let err = ErrorResult::new(
        "E001",
        "msg",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "fix".into(),
            description: "run fix".into(),
        }],
    )
    .require("value");
    let failed = Envelope::error("tool-b/2.0", "tool-b/1.0.0", err, vec![]);
    assert_eq!(failed.envelope_version, "tool-b/2.0");
}

/// A caller-supplied cli_version is distinct from what genesis-vibes' own
/// CARGO_PKG_VERSION would be — callers own their version stamp.
#[test]
fn test_cli_version_is_not_genesis_package_version() {
    let caller_version = "my-custom-tool/0.1.0";
    let env = Envelope::success("0.1", caller_version, EnvelopeKind::Ok, (), vec![], vec![]);
    assert_eq!(env.cli_version, caller_version);
    assert_ne!(env.cli_version, env!("CARGO_PKG_VERSION"));
}
