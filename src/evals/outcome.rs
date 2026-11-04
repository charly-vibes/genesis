//! Envelope parsing and the outcome vocabulary.

/// Error raised while asserting against captured tool output.
use serde_json::Value;

use super::taxonomy::ErrorTaxonomy;

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

/// The observable outcome of a tool invocation, parsed from its stdout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeOutcome {
    /// `ok: true`.
    Ok,
    /// `ok: false`, with the error `code` (if present) and the `command`
    /// fields of any emitted hints.
    Error {
        /// `data.code` from the error envelope, when present.
        code: Option<String>,
        /// `command` of each hint entry, in order.
        hint_commands: Vec<String>,
    },
}

/// Parse a tool's captured stdout into an [`EnvelopeOutcome`].
///
/// Lenient by design: only `ok` is required, so the assertion holds across
/// envelope-version drift. Hints are read from the optional `hints` array.
pub fn parse_envelope(stdout: &str) -> Result<EnvelopeOutcome, EvalsError> {
    let value: Value = serde_json::from_str(stdout).map_err(EvalsError::NotJson)?;
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or(EvalsError::NoOkField)?;
    if ok {
        return Ok(EnvelopeOutcome::Ok);
    }
    let code = value
        .pointer("/data/code")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let hint_commands = value
        .get("hints")
        .and_then(Value::as_array)
        .map(|hints| {
            hints
                .iter()
                .filter_map(|h| h.get("command").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    Ok(EnvelopeOutcome::Error {
        code,
        hint_commands,
    })
}

// ---------------------------------------------------------------------------
// Scenario spec
// ---------------------------------------------------------------------------
