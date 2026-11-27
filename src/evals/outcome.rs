//! Envelope parsing and the outcome vocabulary.
//!
//! The lenient reader itself lives in [`crate::envelope`] (genesis-la4):
//! this module re-exports it for compatibility and adapts its error type
//! onto [`EvalsError`].

use serde_json::Value;

pub use crate::envelope::EnvelopeOutcome;

/// Error raised while asserting against captured tool output.
#[derive(Debug, thiserror::Error)]
pub enum EvalsError {
    /// Captured stdout is not valid JSON.
    #[error("stdout is not valid JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    /// Captured stdout is JSON but has no usable `ok` field.
    #[error("stdout JSON has no boolean `ok` field")]
    NoOkField,
    /// Fixture materialization failed.
    #[error("failed to materialize eval fixture: {0}")]
    Fixture(#[source] crate::fixture::FixtureError),
}

impl From<crate::envelope::EnvelopeParseError> for EvalsError {
    fn from(e: crate::envelope::EnvelopeParseError) -> Self {
        match e {
            crate::envelope::EnvelopeParseError::NotJson(err) => EvalsError::NotJson(err),
            crate::envelope::EnvelopeParseError::NoOkField => EvalsError::NoOkField,
        }
    }
}

/// Parse a tool's captured stdout into an [`EnvelopeOutcome`].
///
/// Compatibility wrapper over [`crate::envelope::parse_envelope`] — the
/// preamble-tolerant lenient reader moved into `envelope` (genesis-la4).
/// Only `ok` is required, so the assertion holds across envelope-version
/// drift; hint commands are the union of both channels (the error channel
/// `data.remediation[]` first, then the legacy top-level `hints` array),
/// with malformed entries counted, not silently dropped.
pub fn parse_envelope(stdout: &str) -> Result<EnvelopeOutcome, EvalsError> {
    crate::envelope::parse_envelope(stdout).map_err(EvalsError::from)
}

/// Value-based sibling of [`parse_envelope`] (genesis-0hx): parses an
/// already-deserialized envelope without a string round-trip.
pub fn parse_envelope_value(value: &Value) -> Result<EnvelopeOutcome, EvalsError> {
    crate::envelope::parse_envelope_value(value).map_err(EvalsError::from)
}
