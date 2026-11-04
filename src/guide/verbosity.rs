//! Progressive-disclosure verbosity levels and their clap `#[command(flatten)]`
//! args struct.

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
