//! The `Output<T>` carrier: data plus warnings plus an optional next-step
//! suggestion, renderable to stderr/stdout or a JSON envelope.

/// A guided CLI output value.
///
use serde::Serialize;
use std::fmt::Debug;
use std::io::Write;

use super::format::OutputFormat;
use super::verbosity::Verbosity;
use crate::suggestions::Suggestion;

pub struct Output<T: Debug> {
    /// The primary payload.
    pub data: T,
    /// Optional next-step suggestion (printed as a "-> Run: ..." footer).
    pub next_step: Option<Suggestion>,
    /// Non-fatal warnings that may be shown at higher verbosity.
    pub warnings: Vec<String>,
    /// Minimum verbosity level at which this output should be shown.
    pub verbosity: u8,
    /// Whether this output represents an error condition.
    pub is_error: bool,
}

impl<T: Debug> Output<T> {
    /// Create a success output.
    ///
    /// Success outputs are shown at Normal verbosity or above (not at Quiet).
    pub fn success(data: T) -> Self {
        Self {
            data,
            next_step: None,
            warnings: Vec::new(),
            verbosity: 1, // Normal — not shown at Quiet
            is_error: false,
        }
    }

    /// Create a failure output.
    ///
    /// The error message is stored in the warnings list so it's always
    /// visible regardless of verbosity.
    pub fn failure(err: impl Into<String>) -> Self
    where
        T: Default,
    {
        Self {
            data: T::default(),
            next_step: None,
            warnings: vec![err.into()],
            verbosity: 0,
            is_error: true,
        }
    }

    /// Set the next-step suggestion (fluent).
    pub fn with_next_step(mut self, hint: impl Into<String>) -> Self {
        self.next_step = Some(Suggestion::fix(hint));
        self
    }

    /// Add a warning message (fluent).
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Set the minimum verbosity threshold (fluent).
    pub fn with_verbosity(mut self, level: u8) -> Self {
        self.verbosity = level;
        self
    }

    /// Print this output to the provided streams, respecting verbosity.
    pub fn print(
        &self,
        current_verbosity: Verbosity,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
    ) -> std::io::Result<()> {
        // Always show errors.
        if self.is_error {
            for warning in &self.warnings {
                writeln!(stderr, "{}", warning)?;
            }
        }

        // Show data if verbosity threshold is met.
        if current_verbosity.should_show(self.verbosity) {
            writeln!(stdout, "{:?}", self.data)?;
        }

        self.print_secondary(current_verbosity, stdout, stderr)?;
        Ok(())
    }

    /// Verbose warnings and the next-step footer (normal level or above).
    fn print_secondary(
        &self,
        current_verbosity: Verbosity,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
    ) -> std::io::Result<()> {
        if current_verbosity >= Verbosity::Verbose {
            for warning in &self.warnings {
                writeln!(stderr, "\u{26a0} {}", warning)?;
            }
        }
        if current_verbosity >= Verbosity::Normal
            && let Some(ref suggestion) = self.next_step
            && let Some(footer) = suggestion.footer()
        {
            writeln!(stderr, "{}", footer)?;
        }
        Ok(())
    }

    /// Convert this output into a JSON envelope for `--json` mode.
    ///
    /// `cli_version` must be the caller's own CLI version (see
    /// [`crate::envelope::Envelope::success`]).
    ///
    /// Requires `T: Serialize`.
    pub fn to_envelope(&self, cli_version: &str) -> crate::envelope::Envelope<&T>
    where
        T: Serialize,
    {
        let kind = if self.is_error {
            crate::envelope::EnvelopeKind::Error
        } else {
            crate::envelope::EnvelopeKind::Ok
        };

        let hints = self.envelope_hints();
        let warnings = self.envelope_warnings();

        crate::envelope::Envelope::success(
            cli_version,
            kind,
            &self.data,
            warnings,
            hints.unwrap_or_default(),
        )
    }

    /// The next-step suggestion as envelope hints, if any.
    ///
    /// genesis-r13: the hint command must be the bare runnable command —
    /// footer() embeds the human '→ Run: ' prefix, which made JSON consumers
    /// receive '→ Run: run: specodelic graph'.
    fn envelope_hints(&self) -> Option<Vec<crate::envelope::HintEntry>> {
        self.next_step.as_ref().and_then(|s| {
            s.command().map(|cmd| {
                vec![crate::envelope::HintEntry {
                    command: cmd.to_string(),
                    description: s.guidance(),
                }]
            })
        })
    }

    /// Map this output's warnings to envelope warnings.
    fn envelope_warnings(&self) -> Vec<crate::envelope::Warning> {
        self.warnings
            .iter()
            .map(|w| crate::envelope::Warning {
                rule_name: String::new(),
                entity_id: None,
                message: w.clone(),
                suggested_remediation: None,
            })
            .collect()
    }

    /// Emit this output in the requested format.
    ///
    /// - `Human` — calls `print()` with the current verbosity.
    /// - `Json` — serializes via `to_envelope()` and writes a JSON line to stdout.
    ///
    /// Errors are always written to stderr regardless of format.
    pub fn emit(
        &self,
        cli_version: &str,
        format: OutputFormat,
        current_verbosity: Verbosity,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
    ) -> std::io::Result<()>
    where
        T: Serialize,
    {
        match format {
            OutputFormat::Human => self.print(current_verbosity, stdout, stderr),
            OutputFormat::Json => {
                // Always print errors to stderr even in JSON mode.
                if self.is_error {
                    for warning in &self.warnings {
                        writeln!(stderr, "{}", warning)?;
                    }
                }
                let envelope = self.to_envelope(cli_version);
                let json = serde_json::to_string(&envelope).map_err(std::io::Error::other)?;
                writeln!(stdout, "{}", json)?;
                Ok(())
            }
        }
    }
}
