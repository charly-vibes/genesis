//! Structured CLI output envelope.
//!
//! Port from `dont/src/envelope.rs` (supersedes dont-2j6o).
//!
//! ## Changes from dont
//!
//! - `EnvelopeKind` was generalized: 15 dont-specific variants removed,
//!   replaced with generic `Ok`/`Error`/`Empty`/`List`/`Check`/`Doctor`/
//!   `Version`/`Stats`/`Info`/`Warning`. `data` is generic over `T: Serialize`.
//! - `envelope_version` is caller-supplied at construction (genesis-la4):
//!   each tool passes its own protocol version (per-tool `&'static str`),
//!   so downstream tools no longer fork the whole envelope module to bump
//!   or read their own version. `cli_version` is likewise caller-supplied.
//!
//! ## Design notes (carried verbatim from dont-2j6o)
//!
//! - The envelope is the **single** output format for all CLI commands:
//!   every command returns an `Envelope<T>`, callers check `ok` first.
//! - `EnvelopeKind` is a closed enum — adding a variant is a deliberate
//!   decision plus a conformance test update.
//! - `ErrorResult` enforces Invariant 3.2.5: `remediation` must be non-empty
//!   (constructor returns `Err` if empty). A bare error message without a
//!   suggested fix is never emitted.
//! - `Meta` carries observability data (duration, tx id, request id, current
//!   author via `set_author`). `Warnings` always collected; `Hints` optional
//!   next-step suggestions; `ephemeral` marks envelopes never persisted/logged.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

mod notice;
mod outcome;
mod receipt;

pub use notice::{HintEntry, RemediationEntry, UnmetClause, Warning};
pub use outcome::{EnvelopeOutcome, EnvelopeParseError, parse_envelope, parse_envelope_value};
pub use receipt::{ReceiptMeta, TerminalOutcome};

static CURRENT_AUTHOR: OnceLock<String> = OnceLock::new();

/// Set the global author for envelope metadata. Must be called once at
/// startup; subsequent calls are silently ignored (the first call wins).
/// If two tools share a process, the first caller owns all envelopes.
pub fn set_author(author: String) {
    let _ = CURRENT_AUTHOR.set(author);
}

/// The author previously installed via [`set_author`], if any. Public so
/// tools sharing a process can observe (and test) who owns envelope
/// metadata without forking the envelope module (genesis-la4).
pub fn current_author() -> Option<String> {
    CURRENT_AUTHOR.get().cloned()
}

/// `ok` must agree with the envelope kind (genesis-r13): failed outputs are
/// routed through `Envelope::success` with kind `Error`, and claiming
/// `ok: true` there would lie to machine consumers.
fn ok_for(kind: EnvelopeKind) -> bool {
    !matches!(kind, EnvelopeKind::Error)
}

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

/// Observability metadata attached to every envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub duration_ms: u64,
    pub tx: Option<u64>,
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
}

impl Meta {
    /// Baseline metadata: zeroed timing, no tx/request id, current author.
    fn fresh() -> Self {
        Self {
            duration_ms: 0,
            tx: None,
            request_id: None,
            author: current_author(),
        }
    }
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
        Self::ensure_remediation(&remediation)?;
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

    /// Invariant 3.2.5: a bare error message without a suggested fix is
    /// never emitted.
    fn ensure_remediation(remediation: &[RemediationEntry]) -> Result<(), &'static str> {
        if remediation.is_empty() {
            return Err("remediation must be non-empty (Invariant 3.2.5)");
        }
        Ok(())
    }
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
    /// `envelope_version` is the caller's own envelope protocol version —
    /// bump it when the caller's envelope shape changes (genesis-la4). It
    /// is distinct from `cli_version`, which identifies the emitting tool.
    /// Both must describe the caller, not genesis-vibes.
    ///
    /// Correct: `Envelope::success(env!("CARGO_PKG_VERSION"), env!("CARGO_PKG_VERSION"), …)`
    /// in your own crate (or a distinct protocol version when the tool
    /// versions its envelope independently).
    /// Wrong: hardcoding another tool's version or anything derived from
    /// genesis-vibes' `Cargo.toml` when you are not genesis.
    pub fn success(
        envelope_version: &'static str,
        cli_version: &str,
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
    ) -> Self {
        Self::success_with_tx(
            envelope_version,
            cli_version,
            kind,
            data,
            warnings,
            hints,
            None,
        )
    }

    /// Envelope shell for the success constructors: every field except
    /// the caller-stamped versions, blank here and never escaping.
    fn shell(kind: EnvelopeKind, data: T, warnings: Vec<Warning>, hints: Vec<HintEntry>) -> Self {
        Self {
            ok: ok_for(kind),
            envelope_version: String::new(),
            cli_version: String::new(),
            envelope_kind: kind,
            data,
            warnings,
            hints: Some(hints),
            ephemeral: None,
            receipt: None,
            meta: Meta::fresh(),
        }
    }

    /// Create a success envelope with a transaction id.
    ///
    /// `envelope_version` and `cli_version` must be the caller's own. See [`Envelope::success`].
    pub fn success_with_tx(
        envelope_version: &'static str,
        cli_version: &str,
        kind: EnvelopeKind,
        data: T,
        warnings: Vec<Warning>,
        hints: Vec<HintEntry>,
        tx: Option<u64>,
    ) -> Self {
        let mut envelope = Self::shell(kind, data, warnings, hints);
        envelope.envelope_version = envelope_version.to_string();
        envelope.cli_version = cli_version.to_string();
        envelope.meta.tx = tx;
        envelope
    }

    /// Attach receipt metadata (builder style, consumes and returns `self`).
    ///
    /// ```
    /// use genesis::envelope::{Envelope, EnvelopeKind, ReceiptMeta, TerminalOutcome};
    ///
    /// let env = Envelope::success(env!("CARGO_PKG_VERSION"), "my-tool/1.0.0", EnvelopeKind::Ok, "done", vec![], vec![])
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
    /// `envelope_version` and `cli_version` must be the caller's own. See [`Envelope::success`].
    pub fn error(
        envelope_version: &'static str,
        cli_version: &str,
        err: ErrorResult,
        warnings: Vec<Warning>,
    ) -> Self {
        Self {
            ok: false,
            envelope_version: envelope_version.to_string(),
            cli_version: cli_version.to_string(),
            envelope_kind: EnvelopeKind::Error,
            data: err,
            warnings,
            hints: None,
            ephemeral: None,
            receipt: None,
            meta: Meta::fresh(),
        }
    }
}

#[cfg(test)]
#[path = "envelope/tests.rs"]
mod tests;
