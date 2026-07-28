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

use crate::feedback::scratch as scratch_mod;
use crate::suggestions::{CommandRegistry, Suggestion};
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

// ---------------------------------------------------------------------------
// ErrorSink
// ---------------------------------------------------------------------------

/// Configuration for self-healing error handling.
///
/// Wires together error printing, scratch persistence, and optional
/// feedback fallback.
#[derive(Debug, Clone)]
pub struct ErrorSink {
    /// The tool name (used for scratch directory and feedback subcommand).
    pub tool_name: String,
    /// Persist the last error to the error scratch.
    pub scratch: bool,
    /// Print the error with a Suggestion::Fix footer.
    pub suggest: bool,
    /// Include the full ContextBundle in the error output.
    pub context: bool,
    /// Name of the feedback subcommand, if the tool has one.
    /// If None, no feedback suggestion is printed.
    pub feedback_subcommand: Option<String>,
    /// The current verbosity level.
    pub verbosity: Verbosity,
}

impl ErrorSink {
    /// Create a new error sink with sensible defaults.
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            scratch: true,
            suggest: true,
            context: false,
            feedback_subcommand: Some("feedback".into()),
            verbosity: Verbosity::Normal,
        }
    }

    /// Handle an error — print it, optionally write to scratch, and
    /// optionally suggest a feedback command.
    pub fn handle(&self, err: &dyn std::error::Error, stderr: &mut impl Write) {
        self.handle_message(&err.to_string(), None, stderr);
    }

    /// Handle an error with an explicit suggestion footer override.
    pub fn handle_with_footer(
        &self,
        err: &dyn std::error::Error,
        suggestion: &Suggestion,
        stderr: &mut impl Write,
    ) {
        self.handle_message(&err.to_string(), Some(suggestion), stderr);
    }

    /// Core error handler implementation.
    fn handle_message(
        &self,
        message: &str,
        suggestion: Option<&Suggestion>,
        stderr: &mut impl Write,
    ) {
        // Always print the error.
        let _ = writeln!(stderr, "{}: {}", self.tool_name, message);

        // Print suggestion footer.
        if let Some(s) = suggestion {
            if let Some(footer) = s.footer() {
                let _ = writeln!(stderr, "{}", footer);
            }
        } else if self.suggest {
            // No explicit suggestion — print generic footer.
            let _ = writeln!(stderr, "-> Run: {} doctor", self.tool_name);
        }

        // Optionally print feedback fallback (only when no suggestion at all).
        if self.feedback_subcommand.is_some() && suggestion.is_none() && !self.suggest {
            let _ = writeln!(
                stderr,
                "Feedback: {} {} bug --from-last-error",
                self.tool_name,
                self.feedback_subcommand.as_deref().unwrap_or("feedback"),
            );
        }

        // Write to scratch if enabled.
        if self.scratch {
            let record = scratch_mod::ErrorRecord {
                ts: chrono_now(),
                argv: vec![self.tool_name.clone()],
                exit: 1,
                footer: suggestion.and_then(|s| s.footer()),
                kind: "error".into(),
            };
            scratch_mod::write_scratch_best_effort(&self.tool_name, &record);
        }
    }
}

/// Get an ISO 8601 timestamp without pulling in chrono.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    // Simple ISO-like format: YYYY-MM-DDTHH:MM:SSZ
    let days = secs / 86400;
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    // Days since epoch (1970-01-01)
    let mut y = 1970i64;
    let mut remaining = days as i64;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        m += 1;
    }
    let d = remaining + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// ---------------------------------------------------------------------------
// GuideBuilder & Guide
// ---------------------------------------------------------------------------

/// A builder for assembling a `Guide`.
///
/// ```rust
/// use genesis::guide::Guide;
///
/// let guide = Guide::builder("my-tool", "0.1.0")
///     .commands(&["init", "check"])
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct GuideBuilder {
    name: String,
    version: String,
    about: Option<String>,
    commands: Vec<String>,
    verbosity: u8,
    has_config: bool,
}

impl GuideBuilder {
    /// Set the tool version.
    pub fn version(mut self, version: &str) -> Self {
        self.version = version.to_string();
        self
    }

    /// Set the tool's one-line description.
    pub fn about(mut self, about: &str) -> Self {
        self.about = Some(about.to_string());
        self
    }

    /// Register valid commands (for typo detection).
    pub fn commands(mut self, commands: &[&str]) -> Self {
        self.commands = commands.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Enable ConfigStore integration.
    ///
    /// No-op if `genesis::config` is not available — this method exists
    /// for API compatibility when downstream tools adopt config later.
    pub fn config<T>(mut self) -> Self
    where
        T: crate::config::ConfigFile + serde::de::DeserializeOwned + 'static,
    {
        self.has_config = true;
        self
    }

    /// Set the maximum verbosity level.
    pub fn max_verbosity(mut self, level: u8) -> Self {
        self.verbosity = level.min(Verbosity::MAX);
        self
    }

    /// Build the `Guide`.
    pub fn build(self) -> Guide {
        let mut registry = CommandRegistry::new();
        if !self.commands.is_empty() {
            registry.register(&self.name, self.commands);
        }

        Guide {
            name: self.name,
            version: self.version,
            about: self.about,
            registry,
            verbosity: Verbosity::from(self.verbosity),
            has_config: self.has_config,
        }
    }
}

/// A coherent CLI scaffold for a guiding tool.
///
/// Assembles all genesis modules into one entry point.
#[derive(Debug, Clone)]
pub struct Guide {
    name: String,
    version: String,
    #[allow(dead_code)]
    about: Option<String>,
    registry: CommandRegistry,
    verbosity: Verbosity,
    #[allow(dead_code)]
    has_config: bool,
}

impl Guide {
    /// Create a new `GuideBuilder`.
    pub fn builder(name: &str, version: &str) -> GuideBuilder {
        GuideBuilder {
            name: name.to_string(),
            version: version.to_string(),
            about: None,
            commands: Vec::new(),
            verbosity: Verbosity::Normal as u8,
            has_config: false,
        }
    }

    /// The tool name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The tool version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// The current verbosity.
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// The command registry.
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Run a command handler, wrapping its `Output`.
    ///
    /// Handles printing, error formatting, and exit code.
    pub fn run<T, F>(&self, f: F) -> i32
    where
        T: Debug,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        match f() {
            Ok(output) => {
                let mut stdout = std::io::stdout();
                let mut stderr = std::io::stderr();
                if let Err(e) = output.print(self.verbosity, &mut stdout, &mut stderr) {
                    let _ = writeln!(&mut stderr, "error printing output: {}", e);
                    return 1;
                }
                if output.is_error { 1 } else { 0 }
            }
            Err(err) => {
                let sink = ErrorSink::new(&self.name).with_verbosity(self.verbosity);
                let mut stderr = std::io::stderr();
                sink.handle(err.as_ref(), &mut stderr);
                1
            }
        }
    }

    /// Create an `ErrorSink` configured for this tool.
    pub fn error_sink(&self) -> ErrorSink {
        ErrorSink::new(&self.name).with_verbosity(self.verbosity)
    }
}

impl ErrorSink {
    /// Set the verbosity level (fluent).
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Enable or disable scratch persistence (fluent).
    pub fn with_scratch(mut self, enabled: bool) -> Self {
        self.scratch = enabled;
        self
    }

    /// Enable or disable suggestion footer (fluent).
    pub fn with_suggest(mut self, enabled: bool) -> Self {
        self.suggest = enabled;
        self
    }

    /// Set the feedback subcommand name (fluent).
    pub fn with_feedback(mut self, name: Option<&str>) -> Self {
        self.feedback_subcommand = name.map(|s| s.to_string());
        self
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

    // -- ErrorSink ---------------------------------------------------------

    #[test]
    fn test_error_sink_defaults() {
        let sink = ErrorSink::new("test-tool");
        assert_eq!(sink.tool_name, "test-tool");
        assert!(sink.scratch);
        assert!(sink.suggest);
        assert!(!sink.context);
        assert_eq!(sink.feedback_subcommand.as_deref(), Some("feedback"));
    }

    #[test]
    fn test_error_sink_fluent_builders() {
        let sink = ErrorSink::new("tool")
            .with_verbosity(Verbosity::Quiet)
            .with_scratch(false)
            .with_suggest(false)
            .with_feedback(None);

        assert_eq!(sink.verbosity, Verbosity::Quiet);
        assert!(!sink.scratch);
        assert!(!sink.suggest);
        assert!(sink.feedback_subcommand.is_none());
    }

    #[test]
    fn test_error_sink_handle_prints_error() {
        let sink = ErrorSink::new("test-tool").with_scratch(false);
        let mut stderr = Vec::new();
        let err = std::io::Error::other("something broke");
        sink.handle(&err, &mut stderr);
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("test-tool"));
        assert!(stderr.contains("something broke"));
        assert!(stderr.contains("Run:"));
    }

    #[test]
    fn test_error_sink_handle_without_suggest() {
        let sink = ErrorSink::new("test-tool")
            .with_scratch(false)
            .with_suggest(false);
        let mut stderr = Vec::new();
        let err = std::io::Error::other("fail");
        sink.handle(&err, &mut stderr);
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("fail"));
        // No suggestion footer
        assert!(!stderr.contains("Run:"));
    }

    #[test]
    fn test_error_sink_handle_with_footer() {
        let sink = ErrorSink::new("tool").with_scratch(false);
        let mut stderr = Vec::new();
        let err = std::io::Error::other("err");
        let suggestion = Suggestion::fix("custom fix");
        sink.handle_with_footer(&err, &suggestion, &mut stderr);
        let stderr = String::from_utf8(stderr).unwrap();
        assert!(stderr.contains("custom fix"));
    }

    // -- GuideBuilder & Guide ----------------------------------------------

    #[test]
    fn test_guide_new_sets_name_and_version() {
        let guide = Guide::builder("my-tool", "1.0.0").build();
        assert_eq!(guide.name(), "my-tool");
        assert_eq!(guide.version(), "1.0.0");
    }

    #[test]
    fn test_guide_with_commands() {
        let guide = Guide::builder("tool", "0.1")
            .commands(&["init", "check", "doctor"])
            .build();

        let all = guide.registry().all();
        assert_eq!(all.len(), 3);
        assert!(all.contains(&"init"));
        assert!(all.contains(&"check"));
        assert!(all.contains(&"doctor"));
    }

    #[test]
    fn test_guide_with_verbosity() {
        let guide = Guide::builder("tool", "0.1").max_verbosity(2).build();
        assert_eq!(guide.verbosity(), Verbosity::Verbose);
    }

    #[test]
    fn test_guide_run_success_returns_zero() {
        let guide = Guide::builder("test", "0.1").build();
        let exit: i32 = guide.run(|| -> Result<Output<&str>, Box<dyn std::error::Error>> {
            Ok(Output::success("ok"))
        });
        assert_eq!(exit, 0);
    }

    #[test]
    fn test_guide_run_failure_returns_one() {
        let guide = Guide::builder("test", "0.1").build();
        let exit: i32 = guide.run(|| -> Result<Output<String>, Box<dyn std::error::Error>> {
            let output: Output<String> = Output::failure("nope");
            Ok(output)
        });
        assert_eq!(exit, 1);
    }

    #[test]
    fn test_guide_run_error_returns_one() {
        let guide = Guide::builder("test", "0.1").build();
        let exit: i32 = guide.run(|| -> Result<Output<String>, Box<dyn std::error::Error>> {
            Err("something went wrong".into())
        });
        assert_eq!(exit, 1);
    }

    #[test]
    fn test_guide_error_sink_is_configured() {
        let guide = Guide::builder("my-tool", "0.1").build();
        let sink = guide.error_sink();
        assert_eq!(sink.tool_name, "my-tool");
    }
}
