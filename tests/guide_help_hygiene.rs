//! Help-hygiene regression tests for the flatten-ready `clap::Args` structs.
//!
//! Clap's `#[command(flatten)]` merges the child struct's doc-derived
//! `about`/`long_about` into the parent command's help. When the doc comment
//! is not verbatim (doesn't start with a code fence), clap_derive joins all
//! lines with spaces, so multi-paragraph rustdoc — and any embedded rust
//! example — collapses into an unreadable prose blob that then dominates the
//! downstream tool's `--help` (genesis-hjy).
//!
//! Contract: `CliVerbosity` and `CliFormat` carry explicit
//! `about`/`long_about`, so embedding them leaks nothing but their own
//! one-line summary into the parent's help.

use clap::{CommandFactory, Parser};
use genesis::guide::{CliFormat, CliVerbosity};

#[derive(Parser)]
#[command(name = "my-tool")]
struct Cli {
    #[command(flatten)]
    verbose: CliVerbosity,
    #[command(flatten)]
    format: CliFormat,
}

/// The genesis-internal doc-comment prose must not leak into a downstream
/// `--help` screen (regression for genesis-hjy).
#[test]
fn flattened_structs_do_not_leak_doc_prose() {
    let mut cmd = Cli::command();
    let help = cmd.render_long_help().to_string();

    // The first-line summary of both doc comments (clap would use it as
    // about/long_about if the structs had no explicit override).
    assert!(
        !help.contains("Clap-derivable args struct"),
        "CliVerbosity/CliFormat doc comment leaked into --help:\n{help}"
    );
}

/// Any leaked rustdoc example would render mangled (newlines collapsed);
/// the code fence marker itself must never appear.
#[test]
fn flattened_structs_do_not_leak_rust_example() {
    let mut cmd = Cli::command();
    let help = cmd.render_long_help().to_string();

    for leaked in ["```rust", "use genesis::guide::CliFormat", "Cli::parse()"] {
        assert!(
            !help.contains(leaked),
            "doc-comment example fragment {leaked:?} leaked into --help:\n{help}"
        );
    }
}
