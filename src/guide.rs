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

#[cfg(test)]
#[path = "guide/tests.rs"]
mod tests;

mod error_sink;
mod format;
mod output;
mod run;
mod verbosity;

pub use error_sink::ErrorSink;
pub use format::{CliFormat, OutputFormat};
pub use output::Output;
pub use run::{Guide, GuideBuilder};
pub use verbosity::{CliVerbosity, Verbosity};
