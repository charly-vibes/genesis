//! Structured CLI output envelope.
//!
//! Port from `dont/src/envelope.rs` (supersedes dont-2j6o).
//!
//! ## Changes from dont
//!
//! - `EnvelopeKind` was generalized: dont-specific variants removed
//!   (`Claim`, `Claims`, `Term`, `TermList`, `All`, `Events`, `Rule`,
//!   `RuleList`, `RuleResult`, `EvidenceCheck`, `Prime`, `Why`, `DontExplain`,
//!   `DontCompletions`, `EvalExport`). Replaced with generic: `Ok`, `Error`,
//!   `Empty`, `List`, `Check`, `Doctor`, `Version`, `Stats`, `Info`, `Warning`.
//! - `data` is generic over `T: Serialize` (already was in dont).
//! - `CLI_VERSION` now reads `env!("CARGO_PKG_VERSION")` from genesis.
//! - `ENVELOPE_VERSION` reset to `"0.1"` for genesis's own versioning.
//!
//! ## Design notes (carried verbatim from dont-2j6o)
//!
//! - The envelope is the **single** output format for all CLI commands.
//!   Every command returns an `Envelope<T>`. Callers check `ok` first.
//! - `EnvelopeKind` is a closed enum — adding a new variant requires a
//!   deliberate decision and a conformance test update.
//! - `ErrorResult` enforces Invariant 3.2.5: `remediation` must be non-empty
//!   (constructor returns `Err` if empty). A bare error message without a
//!   suggested fix is never emitted.
//! - `Meta` carries observability data: duration, transaction id, request id,
//!   and the current author (set once at startup via `set_author`).
//! - `Warnings` are always collected; even on success, warnings may be present.
//! - `Hints` are optional — they are "next-step" suggestions for the user.
//! - `ephemeral` flag indicates the envelope should not be persisted/logged.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

static CURRENT_AUTHOR: OnceLock<String> = OnceLock::new();

/// Set the global author for envelope metadata.
///
/// Must be called once at startup. Subsequent calls are silently ignored
/// (the first call wins). If two tools share the same process, the first
/// tool to call `set_author` sets the author for all subsequent envelopes.
pub fn set_author(author: String) {
    let _ = CURRENT_AUTHOR.set(author);
}

fn current_author() -> Option<String> {
    CURRENT_AUTHOR.get().cloned()
}

/// Envelope protocol version — bump when the shape changes.
pub const ENVELOPE_VERSION: &str = "0.1";

/// CLI version, injected at compile time from `Cargo.toml`.
pub const CLI_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Discriminator for the kind of data carried in the envelope.
///
/// Each variant maps to a distinct command or response type.
/// Add new variants only when a new command needs a distinct discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeKind {
    /// Generic success response.
    Ok,
    /// Error response — `data` is an `ErrorResult`.
    Error,
    /// Empty response (no data, just status).
    Empty,
    /// List of items.
    List,
    /// Check result.
    Check,
    /// Doctor diagnostic result.
    Doctor,
    /// Version information.
    Version,
    /// Operational statistics.
    Stats,
    /// Informational message.
    Info,
    /// Warning-only response.
    Warning,
}

/// A structured warning associated with a rule or check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub rule_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_remediation: Option<String>,
}

/// A suggested command the user can run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationEntry {
    pub command: String,
    pub description: String,
}

/// An unmet clause within an error result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmetClause {
    pub clause: String,
    pub fix: String,
}

/// Observability metadata attached to every envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub duration_ms: u64,
    pub tx: Option<u64>,
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

/// Structured error result, carrying remediation steps.
///
/// Invariant 3.2.5: `remediation` must be non-empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResult {
    pub code: String,
    pub message: String,
    pub rule_name: Option<String>,
    pub spec_ref: Option<String>,
    pub entity_id: Option<String>,
    pub unmet_clauses: Vec<UnmetClause>,
    pub remediation: Vec<RemediationEntry>,
}

impl ErrorResult {
    /// Create a new `ErrorResult`.
    ///
    /// Returns `Err` if `remediation` is empty (Invariant 3.2.5).
    pub fn new(
        code: &str,
        message: &str,
        rule_name: Option<&str>,
        spec_ref: Option<&str>,
        entity_id: Option<&str>,
        unmet_clauses: Vec<UnmetClause>,
        remediation: Vec<RemediationEntry>,
    ) -> Result<Self, &'static str> {
        if remediation.is_empty() {
            return Err("remediation must be non-empty (Invariant 3.2.5)");
        }
        Ok(Self {
            code: code.to_string(),
            message: message.to_string(),
            rule_name: rule_name.map(str::to_string),
            spec_ref: spec_ref.map(str::to_string),
            entity_id: entity_id.map(str::to_string),
            unmet_clauses,
            remediation,
        })
    }
}

/// A hint entry — a suggested next command for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintEntry {
    pub command: String,
    pub description: String,
}

/// The universal CLI output envelope.
///
/// Every command returns this. Callers check `ok` first, then inspect `data`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T: Serialize> {
    pub ok: bool,
    pub envelope_version: String,
    pub cli_version: String,
    pub envelope_kind: EnvelopeKind,
    pub data: T,
    pub warnings: Vec<Warning>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hints: Option<Vec<HintEntry>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ephemeral: Option<bool>,
    pub meta: Meta,
}

impl<T: Serialize> Envelope<T> {
    /// Create a success envelope.
    pub fn success(
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
    ) -> Self {
        Self {
            ok: true,
            envelope_version: ENVELOPE_VERSION.to_string(),
            cli_version: CLI_VERSION.to_string(),
            envelope_kind: kind,
            data,
            warnings,
            hints: Some(hints),
            ephemeral: None,
            meta: Meta {
                duration_ms: 0,
                tx: None,
                request_id: None,
                author: current_author(),
            },
        }
    }

    /// Create a success envelope with a transaction id.
    pub fn success_with_tx(
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
        tx: Option<u64>,
    ) -> Self {
        let mut env = Self::success(kind, data, warnings, hints);
        env.meta.tx = tx;
        env
    }
}

impl Envelope<ErrorResult> {
    /// Create an error envelope.
    pub fn error(err: ErrorResult, warnings: Vec<Warning>) -> Self {
        Self {
            ok: false,
            envelope_version: ENVELOPE_VERSION.to_string(),
            cli_version: CLI_VERSION.to_string(),
            envelope_kind: EnvelopeKind::Error,
            data: err,
            warnings,
            hints: None,
            ephemeral: None,
            meta: Meta {
                duration_ms: 0,
                tx: None,
                request_id: None,
                author: current_author(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // ── Envelope shape conformance ────────────────────────────────────

    #[test]
    fn test_success_envelope_has_required_fields() {
        let env = Envelope::success(EnvelopeKind::Ok, "hello", vec![], vec![]);

        assert!(env.ok, "success envelope must have ok=true");
        assert_eq!(env.envelope_version, "0.1");
        assert_eq!(env.cli_version, env!("CARGO_PKG_VERSION"));
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
        .unwrap();

        let env = Envelope::error(err, vec![]);

        assert!(!env.ok, "error envelope must have ok=false");
        assert_eq!(env.envelope_kind, EnvelopeKind::Error);
        assert_eq!(env.data.code, "E001");
        assert_eq!(env.data.message, "something went wrong");
        assert!(env.hints.is_none());
    }

    #[test]
    fn test_envelope_serializes_to_json() {
        let env = Envelope::success(EnvelopeKind::Ok, "hello", vec![], vec![]);

        let json = serde_json::to_string(&env).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["ok"], true);
        assert_eq!(parsed["envelope_version"], "0.1");
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
        .unwrap();

        let env = Envelope::error(err, vec![]);
        let json = serde_json::to_string_pretty(&env).unwrap();

        assert!(json.contains("E001"));
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
            let json = serde_json::to_string(&kind).unwrap();
            assert_eq!(json, format!("\"{}\"", expected), "variant {:?}", kind);
        }
    }

    // ── Author tracking ───────────────────────────────────────────────

    #[test]
    fn test_set_author_appears_in_meta() {
        set_author("test-user".into());
        let env = Envelope::success(EnvelopeKind::Ok, (), vec![], vec![]);
        assert_eq!(env.meta.author.as_deref(), Some("test-user"));
    }

    // ── Success with tx ───────────────────────────────────────────────

    #[test]
    fn test_success_with_tx_includes_tx_in_meta() {
        let env = Envelope::success_with_tx(EnvelopeKind::Ok, (), vec![], vec![], Some(42));
        assert_eq!(env.meta.tx, Some(42));
    }

    #[test]
    fn test_success_with_tx_none_omits_tx() {
        let env = Envelope::success_with_tx(EnvelopeKind::Ok, (), vec![], vec![], None);
        assert!(env.meta.tx.is_none());
    }
}
