//! `GuideBuilder` and `Guide`: assemble commands/sections and dispatch the
//! run to the requested output format.

use std::io::Write;

use crate::suggestions::CommandRegistry;

use super::error_sink::ErrorSink;
use super::format::OutputFormat;
use super::output::Output;
use super::verbosity::Verbosity;

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
        let registry = build_registry(&self.name, self.commands);

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

/// A registry pre-populated with the guide's commands, if any.
fn build_registry(name: &str, commands: Vec<String>) -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    if !commands.is_empty() {
        registry.register(name, commands);
    }
    registry
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
        T: std::fmt::Debug,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        self.run_with_writers(f, &mut std::io::stdout(), &mut std::io::stderr())
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
        T: std::fmt::Debug,
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
        T: std::fmt::Debug + serde::Serialize,
        F: FnOnce() -> Result<Output<T>, Box<dyn std::error::Error>>,
    {
        self.run_formatted_with_writers(format, f, &mut std::io::stdout(), &mut std::io::stderr())
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
        T: std::fmt::Debug + serde::Serialize,
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
