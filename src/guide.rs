//! CLI scaffold for building guiding tools.
//!
//! Provides `Verbosity`, `Output`, and plumbing to assemble
//! genesis modules into a coherent CLI experience.
//!
//! ## Usage
//!
//! ```rust
//! use genesis::guide::{Verbosity, Output};
//!
//! let output = Output::success("done")
//!     .with_next_step("Run doctor")
//!     .with_warning("check your config");
//!
//! let verbosity = Verbosity::from(2u8);
//! let mut out = std::io::stdout();
//! let mut err = std::io::stderr();
//! output.print(verbosity, &mut out, &mut err).unwrap();
//! ```

use crate::suggestions::Suggestion;
use serde::Serialize;
use std::fmt::Debug;
use std::io::Write;

/// Progressive-disclosure verbosity level.
///
/// Maps to clap's `-v` count:
/// - `0` = Quiet (errors only)
/// - `1` = Normal (default — result + next step)
/// - `2` = Verbose (+warnings + context)
/// - `3+` = Debug (+internals + trace)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet = 0,
    Normal = 1,
    Verbose = 2,
    Debug = 3,
}

impl Verbosity {
    /// The maximum verbosity level we recognise.
    pub const MAX: u8 = 3;

    /// Returns `true` if `level` should be displayed at this verbosity.
    ///
    /// A level is shown when the output's verbosity threshold is <= the
    /// current verbosity.
    pub fn should_show(&self, level: u8) -> bool {
        *self as u8 >= level
    }

    /// Return the numeric value.
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

impl From<u8> for Verbosity {
    fn from(value: u8) -> Self {
        match value {
            0 => Verbosity::Quiet,
            1 => Verbosity::Normal,
            2 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }
}

impl std::fmt::Display for Verbosity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verbosity::Quiet => write!(f, "quiet"),
            Verbosity::Normal => write!(f, "normal"),
            Verbosity::Verbose => write!(f, "verbose"),
            Verbosity::Debug => write!(f, "debug"),
        }
    }
}

/// A guided CLI output value.
///
/// Every command handler returns `Output<T>`. The `Guide` runner prints it
/// with appropriate formatting, verbosity filtering, and envelope wrapping.
#[derive(Debug, Clone)]
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

        // Show warnings at verbose level or above.
        if current_verbosity >= Verbosity::Verbose {
            for warning in &self.warnings {
                writeln!(stderr, "\u{26a0} {}", warning)?;
            }
        }

        // Show next-step footer at normal level or above.
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
    /// Requires `T: Serialize`.
    pub fn to_envelope(&self) -> crate::envelope::Envelope<&T>
    where
        T: Serialize,
    {
        let kind = if self.is_error {
            crate::envelope::EnvelopeKind::Error
        } else {
            crate::envelope::EnvelopeKind::Ok
        };

        let hints = self.next_step.as_ref().and_then(|s| {
            s.footer().map(|footer| {
                vec![crate::envelope::HintEntry {
                    command: footer,
                    description: s.message(),
                }]
            })
        });

        let warnings: Vec<crate::envelope::Warning> = self
            .warnings
            .iter()
            .map(|w| crate::envelope::Warning {
                rule_name: String::new(),
                entity_id: None,
                message: w.clone(),
                suggested_remediation: None,
            })
            .collect();

        crate::envelope::Envelope::success(kind, &self.data, warnings, hints.unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Verbosity ---------------------------------------------------------

    #[test]
    fn test_verbosity_from_u8() {
        assert_eq!(Verbosity::from(0u8), Verbosity::Quiet);
        assert_eq!(Verbosity::from(1u8), Verbosity::Normal);
        assert_eq!(Verbosity::from(2u8), Verbosity::Verbose);
        assert_eq!(Verbosity::from(3u8), Verbosity::Debug);
        assert_eq!(Verbosity::from(4u8), Verbosity::Debug);
        assert_eq!(Verbosity::from(255u8), Verbosity::Debug);
    }

    #[test]
    fn test_verbosity_should_show() {
        let quiet = Verbosity::Quiet;
        assert!(!quiet.should_show(1));
        assert!(!quiet.should_show(2));

        let normal = Verbosity::Normal;
        assert!(normal.should_show(0));
        assert!(normal.should_show(1));
        assert!(!normal.should_show(2));

        let verbose = Verbosity::Verbose;
        assert!(verbose.should_show(0));
        assert!(verbose.should_show(1));
        assert!(verbose.should_show(2));
        assert!(!verbose.should_show(3));

        let debug = Verbosity::Debug;
        assert!(debug.should_show(0));
        assert!(debug.should_show(1));
        assert!(debug.should_show(2));
        assert!(debug.should_show(3));
    }

    #[test]
    fn test_verbosity_as_u8() {
        assert_eq!(Verbosity::Quiet.as_u8(), 0);
        assert_eq!(Verbosity::Normal.as_u8(), 1);
        assert_eq!(Verbosity::Verbose.as_u8(), 2);
        assert_eq!(Verbosity::Debug.as_u8(), 3);
    }

    #[test]
    fn test_verbosity_display() {
        assert_eq!(format!("{}", Verbosity::Quiet), "quiet");
        assert_eq!(format!("{}", Verbosity::Normal), "normal");
        assert_eq!(format!("{}", Verbosity::Verbose), "verbose");
        assert_eq!(format!("{}", Verbosity::Debug), "debug");
    }

    // -- Output construction -----------------------------------------------

    #[test]
    fn test_output_success() {
        let output: Output<i32> = Output::success(42);
        assert_eq!(output.data, 42);
        assert!(!output.is_error);
        assert!(output.next_step.is_none());
        assert!(output.warnings.is_empty());
        assert_eq!(output.verbosity, 1);
    }

    #[test]
    fn test_output_failure() {
        let output: Output<String> = Output::failure("something broke");
        assert!(output.is_error);
        assert_eq!(output.warnings, vec!["something broke"]);
        assert!(output.next_step.is_none());
    }

    #[test]
    fn test_output_with_next_step() {
        let output: Output<&str> = Output::success("done").with_next_step("run doctor");
        assert!(output.next_step.is_some());
        let suggestion = output.next_step.as_ref().unwrap();
        let footer = suggestion.footer().unwrap();
        assert!(footer.contains("run doctor"));
    }

    #[test]
    fn test_output_with_warning() {
        let output: Output<&str> = Output::success("ok").with_warning("check config");
        assert_eq!(output.warnings, vec!["check config"]);
    }

    #[test]
    fn test_output_with_verbosity() {
        let output: Output<&str> = Output::success("data").with_verbosity(2);
        assert_eq!(output.verbosity, 2);
    }

    #[test]
    fn test_output_fluent_chaining() {
        let output: Output<&str> = Output::success("result")
            .with_next_step("next")
            .with_warning("warn1")
            .with_warning("warn2")
            .with_verbosity(1);

        assert_eq!(output.data, "result");
        assert!(output.next_step.is_some());
        assert_eq!(output.warnings.len(), 2);
        assert_eq!(output.verbosity, 1);
    }

    // -- Output::print -----------------------------------------------------

    #[test]
    fn test_print_success_at_normal_verbosity() {
        let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        output
            .print(Verbosity::Normal, &mut stdout, &mut stderr)
            .unwrap();

        let stdout = String::from_utf8(stdout).unwrap();
        let stderr = String::from_utf8(stderr).unwrap();

        assert!(stdout.contains("hello"));
        assert!(stderr.contains("run doctor"));
    }

    #[test]
    fn test_print_success_at_quiet_verbosity() {
        let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        output
            .print(Verbosity::Quiet, &mut stdout, &mut stderr)
            .unwrap();

        let stdout = String::from_utf8(stdout).unwrap();
        let stderr = String::from_utf8(stderr).unwrap();

        // Quiet shows nothing
        assert!(stdout.is_empty());
        assert!(stderr.is_empty());
    }

    #[test]
    fn test_print_failure_always_shows() {
        let output: Output<String> = Output::failure("critical error");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        output
            .print(Verbosity::Quiet, &mut stdout, &mut stderr)
            .unwrap();

        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("critical error"));
    }

    #[test]
    fn test_print_warnings_only_at_verbose() {
        let output: Output<&str> = Output::success("data").with_warning("a warning");
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        // Normal verbosity -- no warnings
        output
            .print(Verbosity::Normal, &mut stdout, &mut stderr)
            .unwrap();
        let stderr_normal = String::from_utf8(stderr.clone()).unwrap();
        assert!(!stderr_normal.contains("a warning"));

        // Verbose verbosity -- warnings shown
        let mut stderr = Vec::new();
        output
            .print(Verbosity::Verbose, &mut stdout, &mut stderr)
            .unwrap();
        let stderr_verbose = String::from_utf8(stderr).unwrap();
        assert!(stderr_verbose.contains("a warning"));
    }

    #[test]
    fn test_print_verbosity_threshold() {
        let output: Output<&str> = Output::success("secret").with_verbosity(2);
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        // Normal (1) is below threshold (2) -- data not shown
        output
            .print(Verbosity::Normal, &mut stdout, &mut stderr)
            .unwrap();
        let stdout_normal = String::from_utf8(stdout).unwrap();
        assert!(!stdout_normal.contains("secret"));

        // Verbose (2) meets threshold -- data shown
        let mut stdout = Vec::new();
        output
            .print(Verbosity::Verbose, &mut stdout, &mut stderr)
            .unwrap();
        let stdout_verbose = String::from_utf8(stdout).unwrap();
        assert!(stdout_verbose.contains("secret"));
    }

    // -- Output::to_envelope -----------------------------------------------

    #[test]
    fn test_to_envelope_success() {
        let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
        let envelope = output.to_envelope();

        assert!(envelope.ok);
        assert_eq!(envelope.data, &"hello");
    }

    #[test]
    fn test_to_envelope_includes_warnings() {
        let output: Output<&str> = Output::success("data").with_warning("warn1");
        let envelope = output.to_envelope();

        assert!(!envelope.warnings.is_empty());
        assert_eq!(envelope.warnings[0].message, "warn1");
    }

    #[test]
    fn test_to_envelope_includes_hints() {
        let output: Output<&str> = Output::success("data").with_next_step("run doctor");
        let envelope = output.to_envelope();

        assert!(envelope.hints.is_some());
        let hints = envelope.hints.unwrap();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_to_envelope_serializes_to_json() {
        let output: Output<&str> = Output::success("hello").with_next_step("run doctor");
        let envelope = output.to_envelope();

        let json = serde_json::to_string(&envelope).unwrap();
        assert!(json.contains("hello"));
        assert!(json.contains("run doctor"));
    }
}
