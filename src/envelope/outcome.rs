//! The lenient envelope reader: [`EnvelopeOutcome`] plus preamble-tolerant
//! parsing of captured tool stdout.
//!
//! Moved from `src/evals/outcome.rs` (genesis-la4) so the reader lives next
//! to the envelope it reads and downstream tools (evallerina, testaruda) can
//! delegate without re-implementing the hint-channel union. `evals`
//! re-exports everything for compatibility.

use serde_json::Value;

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
        /// Hint/remediation entries that named no command (neither an
        /// object carrying `command` nor a bare string) and were dropped
        /// from `hint_commands`. Surfaced so fault messages can report
        /// what was discarded instead of dropping it silently (genesis-0hx).
        malformed_hints: usize,
    },
}

/// Failure while reading a captured envelope.
#[derive(Debug, thiserror::Error)]
pub enum EnvelopeParseError {
    /// The payload (stdout from its first `{`, or the whole input when no
    /// `{` is present) is not valid JSON.
    #[error("stdout is not valid JSON: {0}")]
    NotJson(#[source] serde_json::Error),
    /// The payload is JSON but has no usable `ok` field.
    #[error("stdout JSON has no boolean `ok` field")]
    NoOkField,
}

/// Parse a tool's captured stdout into an [`EnvelopeOutcome`].
///
/// Preamble-tolerant (genesis-la4): tools sometimes print human lines
/// (progress, notices) before the envelope, so the first `{` anywhere in
/// stdout starts the payload — free-form agent text is never scored, only
/// what the tool printed, and this stays a process-boundary read.
///
/// Lenient by design: only `ok` is required, so the assertion holds across
/// envelope-version drift. Hint commands are the union of both channels:
/// the error channel (`data.remediation[]`) and the legacy top-level
/// `hints` array. Each entry may be an object carrying a `command` field
/// or a bare string naming the command. The union is stable —
/// error-channel commands first, then legacy hints — and neither channel
/// is discarded. Envelopes with neither channel yield zero commands.
pub fn parse_envelope(stdout: &str) -> Result<EnvelopeOutcome, EnvelopeParseError> {
    let payload = match stdout.find('{') {
        Some(start) => &stdout[start..],
        None => stdout,
    };
    let value: Value = serde_json::from_str(payload).map_err(EnvelopeParseError::NotJson)?;
    parse_envelope_value(&value)
}

/// Parse an already-deserialized envelope into an [`EnvelopeOutcome`]
/// (genesis-0hx).
///
/// Callers holding a `serde_json::Value` (e.g. one entry of a JSONL
/// capture stream) skip the Value→String→Value round-trip — and its
/// unavoidable `expect` inside check paths.
pub fn parse_envelope_value(value: &Value) -> Result<EnvelopeOutcome, EnvelopeParseError> {
    let ok = value
        .get("ok")
        .and_then(Value::as_bool)
        .ok_or(EnvelopeParseError::NoOkField)?;
    if ok {
        return Ok(EnvelopeOutcome::Ok);
    }
    let (hint_commands, malformed_hints) = hint_commands(value);
    Ok(EnvelopeOutcome::Error {
        code: error_code(value),
        hint_commands,
        malformed_hints,
    })
}

/// `data.code` from an error envelope, when present.
fn error_code(value: &Value) -> Option<String> {
    value
        .pointer("/data/code")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Array at a JSON pointer, empty when absent or not an array.
fn array_at<'a>(value: &'a Value, pointer: &str) -> &'a [Value] {
    value
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Hint commands from an error envelope: the union of both channels —
/// `data.remediation[]` first, then the legacy top-level `hints` array.
/// Each entry may be an object carrying a `command` field or a bare
/// string naming the command. Neither channel is discarded; an envelope
/// with neither channel yields an empty vec. Returns the commands plus
/// the number of malformed entries (naming no command) that were dropped.
fn hint_commands(value: &Value) -> (Vec<String>, usize) {
    let error_channel = array_at(value, "/data/remediation");
    let legacy_channel = array_at(value, "/hints");
    let mut commands = Vec::new();
    let mut malformed = 0;
    for entry in error_channel.iter().chain(legacy_channel) {
        match hint_command(entry) {
            Some(command) => commands.push(command),
            None => malformed += 1,
        }
    }
    (commands, malformed)
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
