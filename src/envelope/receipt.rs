//! Receipt metadata for envelopes: terminal outcome, retry identity, and
//! user-visible evidence of a command run (add-aix-eval-loop D1).

use serde::{Deserialize, Serialize};

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
/// Attached to an envelope via
/// [`Envelope::with_receipt`](super::Envelope::with_receipt); tools decide
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
