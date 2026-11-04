//! TTY-aware output format selection (JSON for pipes, Human for terminals).

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
