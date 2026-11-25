//! Unit tests for `envelope` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget).

// pretender: role=test
#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use serde_json;

// ── Envelope shape conformance ────────────────────────────────────

#[test]
fn test_success_envelope_has_required_fields() {
    let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Ok, "hello", vec![], vec![]);

    assert!(env.ok, "success envelope must have ok=true");
    assert_eq!(env.envelope_version, "0.1");
    assert_eq!(env.cli_version, "my-tool/1.0.0");
    assert_eq!(env.envelope_kind, EnvelopeKind::Ok);
    assert_eq!(env.data, "hello");
    assert!(env.warnings.is_empty());
    assert!(env.hints.is_some_and(|h| h.is_empty()));
    assert!(env.ephemeral.is_none());
    assert_eq!(env.meta.duration_ms, 0);
    assert!(env.meta.tx.is_none());
    assert!(env.meta.request_id.is_none());
}

#[test]
fn test_error_envelope_has_required_fields() {
    let err = ErrorResult::new(
        "E001",
        "something went wrong",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "just fix".into(),
            description: "run the fix".into(),
        }],
    )
    .require("value");

    let env = Envelope::error("my-tool/1.0.0", err, vec![]);

    assert!(!env.ok, "error envelope must have ok=false");
    assert_eq!(env.envelope_kind, EnvelopeKind::Error);
    assert_eq!(env.data.code, "E001");
    assert_eq!(env.data.message, "something went wrong");
    assert_eq!(env.cli_version, "my-tool/1.0.0");
    assert!(env.hints.is_none());
}

// genesis-r13: `Output::to_envelope` routes failed outputs through
// `Envelope::success` with kind=Error — the ok field must agree with
// the kind. An error-kind envelope claiming ok:true is a lie to
// machine consumers.
#[test]
fn test_error_kind_carries_ok_false() {
    let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Error, "oops", vec![], vec![]);
    assert!(!env.ok, "envelope with kind=Error must have ok=false");
    let json = serde_json::to_string(&env).require("value");
    assert!(
        json.contains("\"ok\":false"),
        "serialized error-kind envelope must carry ok:false — got: {json}"
    );
}

#[test]
fn test_envelope_serializes_to_json() {
    let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Ok, "hello", vec![], vec![]);

    let json = serde_json::to_string(&env).require("value");
    let parsed: serde_json::Value = serde_json::from_str(&json).require("value");

    assert_eq!(parsed["ok"], true);
    assert_eq!(parsed["envelope_version"], "0.1");
    assert_eq!(parsed["cli_version"], "my-tool/1.0.0");
    assert_eq!(parsed["envelope_kind"], "ok");
    assert_eq!(parsed["data"], "hello");
    assert!(parsed.get("hints").is_some()); // serialized even if empty
    assert!(parsed.get("ephemeral").is_none()); // skipped
}

#[test]
fn test_error_envelope_serializes_properly() {
    let err = ErrorResult::new(
        "E001",
        "failed",
        Some("rule-1"),
        Some("spec-1"),
        Some("entity-1"),
        vec![UnmetClause {
            clause: "must be > 0".into(),
            fix: "provide a positive value".into(),
        }],
        vec![RemediationEntry {
            command: "fix --force".into(),
            description: "apply fix".into(),
        }],
    )
    .require("value");

    let env = Envelope::error("my-tool/1.0.0", err, vec![]);
    let json = serde_json::to_string_pretty(&env).require("value");

    assert!(json.contains("E001"));
    assert!(json.contains("my-tool/1.0.0"));
    assert!(json.contains("rule-1"));
    assert!(json.contains("must be > 0"));
    assert!(json.contains("fix --force"));
}

// ── ErrorResult invariants ────────────────────────────────────────

#[test]
fn test_error_result_requires_remediation() {
    let result = ErrorResult::new("E001", "msg", None, None, None, vec![], vec![]);
    assert!(result.is_err(), "empty remediation must fail");
}

#[test]
fn test_error_result_with_remediation_succeeds() {
    let result = ErrorResult::new(
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
    );
    assert!(result.is_ok());
}

// ── EnvelopeKind variants ─────────────────────────────────────────

#[test]
fn test_envelope_kind_variants_serialize_snake_case() {
    let cases = vec![
        (EnvelopeKind::Ok, "ok"),
        (EnvelopeKind::Error, "error"),
        (EnvelopeKind::Empty, "empty"),
        (EnvelopeKind::List, "list"),
        (EnvelopeKind::Check, "check"),
        (EnvelopeKind::Doctor, "doctor"),
        (EnvelopeKind::Version, "version"),
        (EnvelopeKind::Stats, "stats"),
        (EnvelopeKind::Info, "info"),
        (EnvelopeKind::Warning, "warning"),
    ];

    for (kind, expected) in cases {
        let json = serde_json::to_string(&kind).require("value");
        assert_eq!(json, format!("\"{}\"", expected), "variant {:?}", kind);
    }
}

// ── Author tracking ───────────────────────────────────────────────

#[test]
fn test_set_author_appears_in_meta() {
    set_author("test-user".into());
    let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Ok, (), vec![], vec![]);
    assert_eq!(env.meta.author.as_deref(), Some("test-user"));
}

// ── Success with tx ───────────────────────────────────────────────

#[test]
fn test_success_with_tx_includes_tx_in_meta() {
    let env = Envelope::success_with_tx(
        "my-tool/1.0.0",
        EnvelopeKind::Ok,
        (),
        vec![],
        vec![],
        Some(42),
    );
    assert_eq!(env.meta.tx, Some(42));
}

#[test]
fn test_success_with_tx_none_omits_tx() {
    let env =
        Envelope::success_with_tx("my-tool/1.0.0", EnvelopeKind::Ok, (), vec![], vec![], None);
    assert!(env.meta.tx.is_none());
}

// ── Caller-supplied CLI version ──────────────────────────────────

#[test]
fn test_success_preserves_caller_cli_version() {
    let env = Envelope::success(
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
    let env = Envelope::error("downstream-tool/2.0.0", err, vec![]);
    assert_eq!(env.cli_version, "downstream-tool/2.0.0");
}

#[test]
fn test_success_with_tx_preserves_caller_cli_version() {
    let env = Envelope::success_with_tx(
        "downstream/3.0.0",
        EnvelopeKind::Ok,
        (),
        vec![],
        vec![],
        Some(1),
    );
    assert_eq!(env.cli_version, "downstream/3.0.0");
}

mod receipt;

#[test]
fn test_cli_version_is_not_genesis_package_version() {
    // Verify that a caller-supplied version is distinct from what
    // genesis-vibes' own CARGO_PKG_VERSION would be.
    let caller_version = "my-custom-tool/0.1.0";
    let env = Envelope::success(caller_version, EnvelopeKind::Ok, (), vec![], vec![]);
    assert_eq!(env.cli_version, caller_version);
    // genesis-vibes' own package version is something like "0.5.0"
    // — the caller's version should not equal that.
    assert_ne!(env.cli_version, env!("CARGO_PKG_VERSION"));
}
