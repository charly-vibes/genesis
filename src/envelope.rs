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
//! - `ENVELOPE_VERSION` reset to `"0.1"` for genesis's own versioning.
//! - `cli_version` is caller-supplied at construction — downstream tools pass their own
//!   CLI version, not genesis-vibes' package version.
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

/// Envelope protocol version — bump when the envelope shape changes. This
/// is distinct from `cli_version`, which identifies the emitting tool.
pub const ENVELOPE_VERSION: &str = "0.1";

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

/// Terminal outcome of a command run, recorded in a [`ReceiptMeta`].
///
/// Every external boundary needs an ending — silence cannot be neutral
/// (B173). Timeout is explicit so a deadline hit is never mistaken for a
/// silent failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalOutcome {
    /// The run completed successfully.
    Success,
    /// The run completed with a failure.
    Failure,
    /// The run hit its deadline. Independent of the envelope's `ok` field:
    /// a delivered result with a timed-out follow-up is representable.
    Timeout,
    /// The run was cancelled before reaching a terminal state.
    Cancelled,
}

/// Receipt metadata recording the terminal outcome, retry identity, and
/// user-visible evidence of a command run (add-aix-eval-loop D1).
///
/// Attached to an envelope via [`Envelope::with_receipt`]; tools decide
/// which commands qualify as mutating and what evidence means for them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptMeta {
    /// How the run ended.
    pub terminal_outcome: TerminalOutcome,
    /// 1-based attempt counter; retried runs of the same mutating command
    /// carry increasing numbers.
    pub attempt: u32,
    /// Optional key shared by attempts of the same logical command, so
    /// duplicates and retries are distinguishable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
    /// Optional verifiable statement of the user-visible edge
    /// ("file X exists at path Y") — not free-form narrative.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
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
    /// Optional receipt metadata — opt-in via [`Envelope::with_receipt`].
    /// Absent from serialized output unless set, so envelopes without a
    /// receipt are byte-identical to pre-receipt output.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub receipt: Option<ReceiptMeta>,
    pub meta: Meta,
}

impl<T: Serialize> Envelope<T> {
    /// Create a success envelope.
    ///
    /// `cli_version` must be the caller's own CLI version (e.g., from `env!("CARGO_PKG_VERSION")`
    /// in the downstream tool's crate). Do **not** pass genesis-vibes' version — this field
    /// identifies the emitting tool, not the genesis library.
    ///
    /// Correct: `Envelope::success(env!("CARGO_PKG_VERSION"), …)` in your own crate.
    /// Wrong: hardcoding `"0.5.0"` or anything derived from genesis-vibes' `Cargo.toml`.
    pub fn success(
        cli_version: &str,
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
    ) -> Self {
        Self {
            // genesis-r13: ok must agree with the envelope kind —
            // Output::to_envelope routes failed outputs through here with
            // kind=Error; claiming ok:true would lie to machine consumers.
            ok: !matches!(kind, EnvelopeKind::Error),
            envelope_version: ENVELOPE_VERSION.to_string(),
            cli_version: cli_version.to_string(),
            envelope_kind: kind,
            data,
            warnings,
            hints: Some(hints),
            ephemeral: None,
            receipt: None,
            meta: Meta {
                duration_ms: 0,
                tx: None,
                request_id: None,
                author: current_author(),
            },
        }
    }

    /// Create a success envelope with a transaction id.
    ///
    /// `cli_version` must be the caller's own CLI version. See [`Envelope::success`].
    pub fn success_with_tx(
        cli_version: &str,
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
        tx: Option<u64>,
    ) -> Self {
        let env = Self::success(cli_version, kind, data, warnings, hints);
        Self {
            meta: Meta { tx, ..env.meta },
            ..env
        }
    }

    /// Attach receipt metadata (builder style, consumes and returns `self`).
    ///
    /// ```
    /// use genesis::envelope::{Envelope, EnvelopeKind, ReceiptMeta, TerminalOutcome};
    ///
    /// let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Ok, "done", vec![], vec![])
    ///     .with_receipt(ReceiptMeta {
    ///         terminal_outcome: TerminalOutcome::Success,
    ///         attempt: 1,
    ///         idempotency_key: None,
    ///         evidence: Some("file exists at /tmp/out.txt".into()),
    ///     });
    /// assert!(env.receipt.is_some());
    /// ```
    pub fn with_receipt(mut self, receipt: ReceiptMeta) -> Self {
        self.receipt = Some(receipt);
        self
    }
}

impl Envelope<ErrorResult> {
    /// Create an error envelope.
    ///
    /// `cli_version` must be the caller's own CLI version. See [`Envelope::success`].
    pub fn error(cli_version: &str, err: ErrorResult, warnings: Vec<Warning>) -> Self {
        Self {
            ok: false,
            envelope_version: ENVELOPE_VERSION.to_string(),
            cli_version: cli_version.to_string(),
            envelope_kind: EnvelopeKind::Error,
            data: err,
            warnings,
            hints: None,
            ephemeral: None,
            receipt: None,
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
#[path = "envelope/tests.rs"]
mod tests;
