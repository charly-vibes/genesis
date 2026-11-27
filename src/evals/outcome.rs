//! Envelope parsing and the outcome vocabulary.

/// Error raised while asserting against captured tool output.
use serde_json::Value;

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
        /// `command` of each hint entry, in order. The list is the union
        /// of both hint channels — the error channel
        /// (`data.remediation[]`) first, then the legacy top-level
        /// `hints` array — so neither channel's command is discarded.
        /// Entries may be objects carrying `command` or bare strings.
        hint_commands: Vec<String>,
    },
}

/// Parse a tool's captured stdout into an [`EnvelopeOutcome`].
///
/// Lenient by design: only `ok` is required, so the assertion holds across
/// envelope-version drift. Hint commands are the union of both channels:
/// the error channel (`data.remediation[]`) and the legacy top-level
/// `hints` array. Each entry may be an object carrying a `command` field
/// or a bare string naming the command. The union is stable —
/// error-channel commands first, then legacy hints — and neither channel
/// is discarded. Envelopes with neither channel yield zero commands.
pub fn parse_envelope(stdout: &str) -> Result<EnvelopeOutcome, EvalsError> {
    let value: Value = serde_json::from_str(stdout).map_err(EvalsError::NotJson)?;
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or(EvalsError::NoOkField)?;
    if ok {
        return Ok(EnvelopeOutcome::Ok);
    }
    Ok(EnvelopeOutcome::Error {
        code: error_code(&value),
        hint_commands: hint_commands(&value),
    })
}

/// `data.code` from an error envelope, when present.
fn error_code(value: &Value) -> Option<String> {
    value
        .pointer("/data/code")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Hint commands from an error envelope: the union of both channels —
/// `data.remediation[]` first, then the legacy top-level `hints` array.
/// Each entry may be an object carrying a `command` field or a bare
/// string naming the command. Neither channel is discarded; an envelope
/// with neither channel yields an empty vec.
fn hint_commands(value: &Value) -> Vec<String> {
    let error_channel = value
        .pointer("/data/remediation")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let legacy_channel = value
        .get("hints")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    error_channel
        .iter()
        .chain(legacy_channel)
        .filter_map(hint_command)
        .collect()
}

/// The command named by a hint/remediation entry: an object carrying
/// `command` or a bare string.
fn hint_command(entry: &Value) -> Option<String> {
    match entry {
        Value::String(command) => Some(command.clone()),
        _ => entry
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

// ---------------------------------------------------------------------------
// Scenario spec
// ---------------------------------------------------------------------------
