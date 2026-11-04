//! CLI scaffold for building guiding tools.
//!
//! Provides `Verbosity`, `Output`, `CliVerbosity`, `OutputFormat`, `CliFormat`,
//! and plumbing to assemble genesis modules into a coherent CLI experience.
//!
//! ## Usage
//!
//! ```rust
//! use genesis::guide::{Verbosity, Output, CliVerbosity};
//!
//! let output = Output::success("done")
//!     .with_next_step("Run doctor")
//!     .with_warning("check your config");
//!
//! // From clap's `-v` count:
//! let verbosity = Verbosity::from_verbose_count(2);
//! let mut out = std::io::stdout();
//! let mut err = std::io::stderr();
//! output.print(verbosity, &mut out, &mut err).unwrap();
//!
//! // Or embed the clap args struct:
//! #[derive(clap::Args)]
//! struct MyCli {
//!     #[command(flatten)]
//!     pub verbose: CliVerbosity,
//! }
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

    /// Create a `Verbosity` from clap's `-v` count.
    ///
    /// This is the canonical way to wire up clap's `ArgAction::Count`:
    ///
    /// ```rust
    /// use genesis::guide::Verbosity;
    ///
    /// // Maps -v=0 -> Normal (default), -v -> Verbose, -vv -> Debug
    /// let v = Verbosity::from_verbose_count(0);
    /// assert_eq!(v, Verbosity::Normal);
    ///
    /// let v = Verbosity::from_verbose_count(2);
    /// assert_eq!(v, Verbosity::Debug);
    /// ```
    ///
    /// The difference from `Verbosity::from(0u8)` (which returns `Quiet`)
    /// is that `from_verbose_count(0)` returns `Normal`, because `-v`
    /// absence should mean default, not silence. Use the separate
    /// `-q`/`--quiet` flag for `Quiet`.
    pub fn from_verbose_count(count: u8) -> Self {
        match count {
            0 => Verbosity::Normal,
            1 => Verbosity::Verbose,
            _ => Verbosity::Debug,
        }
    }

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

    /// Return a help footer hinting at progressive-disclosure verbosity.
    ///
    /// Wai's convention: `-v` for advanced options, `-vv` for environment
    /// variables, `-vvv` for internals.
    pub fn help_footer() -> &'static str {
        "Use -v for advanced options, -vv for environment variables, -vvv for internals."
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

// ---------------------------------------------------------------------------
// CliVerbosity
// ---------------------------------------------------------------------------

/// Clap-derivable args struct for `-v`/`-vv`/`-vvv` / `-q`.
///
/// Requires your crate to depend on `clap` with the `derive` feature
/// (already satisfied if you depend on `genesis`, which pulls in clap).
///
/// Embed this in your clap CLI struct with `#[command(flatten)]`:
///
/// ```rust,no_run
/// use clap::Parser;
/// use genesis::guide::CliVerbosity;
///
/// #[derive(Parser)]
/// struct Cli {
///     #[command(flatten)]
///     pub verbose: CliVerbosity,
/// }
///
/// let cli = Cli::parse();
/// let verbosity = cli.verbose.verbosity();
/// ```
///
/// Both flags are marked `global = true` so they are accepted before
/// or after any subcommand, matching wai's CLI ergonomics.
///
/// The `-q`/`--quiet` flag overrides the `-v` count: if `--quiet` is set,
/// verbosity is `Quiet` regardless of `-v`.
// Explicit about/long_about keep this doc comment out of downstream
// `--help` screens: clap flatten merges the child's doc-derived help into
// the parent, and non-verbatim rustdoc collapses to mangled prose there
// (genesis-hjy).
#[derive(Debug, Clone, Copy, clap::Args)]
#[command(
    about = "Verbosity flags (-v/-vv/-vvv/-q)",
    long_about = "Verbosity flags: `-v`/`-vv`/`-vvv` raise verbosity, `-q`/`--quiet` suppresses non-error output (--quiet wins over -v)."
)]
pub struct CliVerbosity {
    /// Increase output verbosity (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error output
    #[arg(short, long, global = true)]
    pub quiet: bool,
}

impl CliVerbosity {
    /// Resolve the effective `Verbosity` from the clap args.
    ///
    /// `--quiet` takes precedence: if set, returns `Quiet`.
    /// Otherwise, `-v` count maps via `Verbosity::from_verbose_count`.
    pub fn verbosity(&self) -> Verbosity {
        if self.quiet {
            Verbosity::Quiet
        } else {
            Verbosity::from_verbose_count(self.verbose)
        }
    }

    /// Return the raw clap `-v` count.
    ///
    /// Use this when you need the full wai-style 3-level progressive
    /// disclosure that `Verbosity` can't express (e.g. `>= 3` checks
    /// for internals vs env vars):
    ///
    /// ```rust,no_run
    /// # use genesis::guide::CliVerbosity;
    /// # let cli = CliVerbosity { verbose: 2, quiet: false };
    /// if cli.raw_count() >= 3 {
    ///     // show internals
    /// } else if cli.raw_count() >= 2 {
    ///     // show env vars
    /// }
    /// ```
    pub fn raw_count(&self) -> u8 {
        self.verbose
    }
}

#[cfg(test)]
mod cli_verbosity_tests {
    use super::*;

    #[test]
    fn default_is_normal() {
        let cv = CliVerbosity {
            verbose: 0,
            quiet: false,
        };
        assert_eq!(cv.verbosity(), Verbosity::Normal);
    }

    #[test]
    fn verbose_count_one() {
        let cv = CliVerbosity {
            verbose: 1,
            quiet: false,
        };
        assert_eq!(cv.verbosity(), Verbosity::Verbose);
    }

    #[test]
    fn verbose_count_two() {
        let cv = CliVerbosity {
            verbose: 2,
            quiet: false,
        };
        assert_eq!(cv.verbosity(), Verbosity::Debug);
    }

    #[test]
    fn verbose_count_three() {
        let cv = CliVerbosity {
            verbose: 3,
            quiet: false,
        };
        assert_eq!(cv.verbosity(), Verbosity::Debug);
    }

    #[test]
    fn quiet_overrides_verbose() {
        let cv = CliVerbosity {
            verbose: 3,
            quiet: true,
        };
        assert_eq!(cv.verbosity(), Verbosity::Quiet);
    }

    #[test]
    fn quiet_without_verbose() {
        let cv = CliVerbosity {
            verbose: 0,
            quiet: true,
        };
        assert_eq!(cv.verbosity(), Verbosity::Quiet);
    }

    #[test]
    fn raw_count_preserves_value() {
        let cv = CliVerbosity {
            verbose: 2,
            quiet: false,
        };
        assert_eq!(cv.raw_count(), 2);

        let cv = CliVerbosity {
            verbose: 3,
            quiet: false,
        };
        assert_eq!(cv.raw_count(), 3);
    }

    #[test]
    fn raw_count_ignores_quiet() {
        let cv = CliVerbosity {
            verbose: 3,
            quiet: true,
        };
        // raw_count returns the clap count regardless of --quiet
        assert_eq!(cv.raw_count(), 3);
    }
}

// ---------------------------------------------------------------------------
// OutputFormat & CliFormat
// ---------------------------------------------------------------------------

/// Output format selection for CLI commands.
///
/// - `Human` — pretty-printed, human-readable output.
/// - `Json` — machine-readable JSON envelope (default for non-TTY).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable, pretty-printed output.
    Human,
    /// Machine-readable JSON envelope.
    Json,
}

/// Clap-derivable args struct for `--json` / `--human`.
///
/// Embed this in your clap CLI struct with `#[command(flatten)]`:
///
/// ```rust,no_run
/// use clap::Parser;
/// use genesis::guide::CliFormat;
///
/// #[derive(Parser)]
/// struct Cli {
///     #[command(flatten)]
///     pub format: CliFormat,
/// }
///
/// let cli = Cli::parse();
/// let fmt = cli.format.format();  // auto-detects TTY vs pipe/agent
/// ```
///
/// When neither `--json` nor `--human` is set, the format is auto-detected:
/// - stdout is a terminal (TTY) → `Human`
/// - stdout is piped or redirected → `Json`
///
/// This ensures agents and CI pipelines always receive machine-readable
/// JSON by default, while humans at a terminal get readable output.
// Explicit about/long_about keep this doc comment out of downstream
// `--help` screens: clap flatten merges the child's doc-derived help into
// the parent, and non-verbatim rustdoc collapses to mangled prose there
// (genesis-hjy).
#[derive(Debug, Clone, Copy, clap::Args)]
#[command(
    about = "Output format flags (--json / --human)",
    long_about = "Output format flags: `--json` emits the JSON envelope, `--human` forces readable text; with neither, the format auto-detects (Human on a TTY, Json piped)."
)]
pub struct CliFormat {
    /// Output machine-readable JSON (default when stdout is not a TTY)
    #[arg(short = 'j', long, global = true)]
    pub json: bool,

    /// Output human-readable text (default when stdout is a TTY)
    #[arg(long, global = true, conflicts_with = "json")]
    pub human: bool,
}

impl CliFormat {
    /// Resolve the effective output format.
    ///
    /// - `--json` → `Json`
    /// - `--human` → `Human`
    /// - Neither → auto-detect: `Human` if stdout is a TTY, `Json` otherwise
    pub fn format(&self) -> OutputFormat {
        if self.json {
            OutputFormat::Json
        } else if self.human {
            OutputFormat::Human
        } else {
            // Auto-detect: JSON for agents/pipes, human for terminals
            use std::io::IsTerminal;
            if std::io::stdout().is_terminal() {
                OutputFormat::Human
            } else {
                OutputFormat::Json
            }
        }
    }

    /// Returns `true` if JSON output is explicitly requested or auto-detected.
    pub fn is_json(&self) -> bool {
        self.json || (!self.human && is_stdout_piped())
    }

    /// Returns `true` if human output is explicitly requested or auto-detected.
    pub fn is_human(&self) -> bool {
        self.human || (!self.json && is_stdout_terminal())
    }
}

fn is_stdout_terminal() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
}

fn is_stdout_piped() -> bool {
    !is_stdout_terminal()
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

        let hints = self.next_step.as_ref().and_then(|s| {
            // genesis-r13: the hint command must be the bare runnable
            // command — footer() embeds the human '→ Run: ' prefix, which
            // made JSON consumers receive '→ Run: run: specodelic graph'.
            s.command().map(|cmd| {
                vec![crate::envelope::HintEntry {
                    command: cmd.to_string(),
                    description: s.guidance(),
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

        crate::envelope::Envelope::success(
            cli_version,
            kind,
            &self.data,
            warnings,
            hints.unwrap_or_default(),
        )
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

    /// Set the maximum verbosity level (by raw count).
    ///
    /// Prefer [`with_verbosity`](GuideBuilder::with_verbosity) when you
    /// already have a resolved `Verbosity` (e.g. from `CliVerbosity`).
    pub fn max_verbosity(mut self, level: u8) -> Self {
        self.verbosity = level.min(Verbosity::MAX);
        self
    }

    /// Set the verbosity from a resolved `Verbosity` value.
    ///
    /// Use this when you've already parsed clap flags via `CliVerbosity`:
    ///
    /// ```rust,no_run
    /// use genesis::guide::{Guide, CliVerbosity};
    /// # use clap::Parser;
    /// # #[derive(Parser)] struct Cli { #[command(flatten)] verbose: CliVerbosity }
    /// # let cli = Cli::parse();
    /// let guide = Guide::builder("my-tool", "0.1.0")
    ///     .with_verbosity(cli.verbose.verbosity())
    ///     .build();
    /// ```
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity.as_u8();
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

    /// Update the verbosity after construction (e.g. from clap args).
    pub fn set_verbosity(&mut self, verbosity: Verbosity) {
        self.verbosity = verbosity;
    }

    /// The command registry.
    pub fn registry(&self) -> &CommandRegistry {
        &self.registry
    }

    /// Run a command handler, wrapping its `Output`.
    ///
    /// Handles printing, error formatting, and exit code.
    ///
    /// # Exit-code contract
    ///
    /// - `exit: 0` — success
    /// - `exit: 1` — user-facing error (handler returned `Err`, or the
    ///   `Output` is an error envelope); stable, backward-compatible
    /// - `exit: 2` — internal failure (I/O error while emitting output)
    /// - panics unwind with Rust's default runtime behavior (exit `101`);
    ///   if the tool opts into `panic = "abort"`, the process aborts
    ///   instead (no `101` guarantee). No panic hook is installed and
    ///   stack traces are never swallowed
    ///
    /// Eval harnesses can distinguish "graceful failure with hint
    /// envelope" (`1`) from "crashed" (nonzero other than `1`). See
    /// `docs/how-to/evals.md` Step 3.
    pub fn run<T, F>(&self, f: F) -> i32
    where
        T: Debug,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        self.run_with_writers(f, &mut stdout, &mut stderr)
    }

    /// Like [`Guide::run`], but writes to the provided streams.
    ///
    /// Enables testing of the internal-failure (`exit: 2`) path without
    /// replacing process-level streams. See [`Guide::run`] for the
    /// exit-code contract.
    ///
    /// Diagnostics on emission failure are best-effort: they are written
    /// to the provided `stderr`, so a broken stderr loses them silently.
    pub fn run_with_writers<T, F>(
        &self,
        f: F,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
    ) -> i32
    where
        T: Debug,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        match f() {
            Ok(output) => {
                if let Err(e) = output.print(self.verbosity, stdout, stderr) {
                    let _ = writeln!(stderr, "error printing output: {}", e);
                    // Internal failure (I/O), not a user-facing error.
                    return 2;
                }
                if output.is_error { 1 } else { 0 }
            }
            Err(err) => {
                let sink = ErrorSink::new(&self.name).with_verbosity(self.verbosity);
                sink.handle(err.as_ref(), stderr);
                1
            }
        }
    }

    /// Run a command handler, dispatching to human or JSON output.
    ///
    /// Like `run()`, but uses `Output::emit()` to respect the requested
    /// output format. See [`Guide::run`] for the exit-code contract
    /// (`exit: 2` marks internal failures here as well).
    pub fn run_formatted<T, F>(&self, format: OutputFormat, f: F) -> i32
    where
        T: Debug + Serialize,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        self.run_formatted_with_writers(format, f, &mut stdout, &mut stderr)
    }

    /// Like [`Guide::run_formatted`], but writes to the provided streams.
    ///
    /// Enables testing of the internal-failure (`exit: 2`) path without
    /// replacing process-level streams. See [`Guide::run`] for the
    /// exit-code contract.
    ///
    /// Diagnostics on emission failure are best-effort: they are written
    /// to the provided `stderr`, so a broken stderr loses them silently.
    pub fn run_formatted_with_writers<T, F>(
        &self,
        format: OutputFormat,
        f: F,
        stdout: &mut impl Write,
        stderr: &mut impl Write,
    ) -> i32
    where
        T: Debug + Serialize,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        match f() {
            Ok(output) => {
                if let Err(e) = output.emit(&self.version, format, self.verbosity, stdout, stderr) {
                    let _ = writeln!(stderr, "error printing output: {}", e);
                    // Internal failure (I/O), not a user-facing error.
                    return 2;
                }
                if output.is_error { 1 } else { 0 }
            }
            Err(err) => {
                let sink = ErrorSink::new(&self.name).with_verbosity(self.verbosity);
                sink.handle(err.as_ref(), stderr);
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
#[cfg(test)]
#[path = "guide/tests.rs"]
mod tests;
