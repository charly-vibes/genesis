//! Deterministic eval primitives for agentic CLI tools.
//!
//! Tools in the charly-vibes suite each hand-roll the same evaluation
//! scaffolding: fixture setup, envelope assertions over captured output,
//! and a way to say *why* a scenario failed. This module is the shared
//! core slice:
//!
//! - [`ErrorTaxonomy`] — the eight cross-tool failure classifications
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
#[path = "evals/tests_e2e.rs"]
mod tests_e2e;

#[cfg(test)]
#[path = "evals/tests_receipt.rs"]
mod tests_receipt;

#[cfg(test)]
#[path = "evals/tests_report.rs"]
mod tests_report;

mod checks;
mod outcome;
mod scenario;
mod taxonomy;

pub use checks::{
    agent_executed_all, agent_followed_hint, doc_drift_blindness, error_envelope_with_hint,
    ok_envelope, receipt_records_terminal_outcome,
};
pub use outcome::{EnvelopeOutcome, EvalsError, parse_envelope, parse_envelope_value};
pub use scenario::{
    AgentStep, CheckOutcome, Distractor, DistractorKind, Scenario, ScenarioCheck, ScenarioReport,
    ScenarioResult,
};
pub use taxonomy::ErrorTaxonomy;
