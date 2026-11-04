//! Deterministic eval primitives for agentic CLI tools.
//!
//! Tools in the charly-vibes suite each hand-roll the same evaluation
//! scaffolding: fixture setup, envelope assertions over captured output,
//! and a way to say *why* a scenario failed. This module is the shared
//! core slice:
//!
//! - [`ErrorTaxonomy`] — the six cross-tool failure classifications
//!   (`ERR_*` codes) from the evaluation-framework research.
//! - Envelope-assertion helpers ([`parse_envelope`]) over captured
//!   subprocess stdout — the portable assertion ("exited nonzero with a
//!   parseable error envelope") holds regardless of exit-code refinement.
//! - [`Scenario`] — fixture setup + agent prompt + deterministic checks,
//!   replayed against a fake-agent transcript ([`AgentStep`]). No live
//!   LLM calls, no subprocess execution environment.
//!
//! Managed-block audit utilities and AIX-ablation provisioning are
//! follow-up slices (see genesis-zxv).

// ---------------------------------------------------------------------------
// Error taxonomy

#[cfg(test)]
#[path = "evals/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "evals/tests_receipt.rs"]
mod tests_receipt;

mod checks;
mod outcome;
mod scenario;
mod taxonomy;

pub use checks::{
    agent_executed_all, agent_followed_hint, doc_drift_blindness, error_envelope_with_hint,
    ok_envelope, receipt_records_terminal_outcome,
};
pub use outcome::{EnvelopeOutcome, EvalsError, parse_envelope};
pub use scenario::{
    AgentStep, CheckOutcome, Distractor, DistractorKind, Scenario, ScenarioCheck, ScenarioReport,
    ScenarioResult,
};
pub use taxonomy::ErrorTaxonomy;
