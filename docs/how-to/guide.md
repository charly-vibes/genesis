# Building a CLI with Guide

**TL;DR:** Use `GuideBuilder` to assemble genesis modules into a coherent CLI with progressive-disclosure verbosity, TTY-aware output format, and auto-generated suggestions.

## Context & Prerequisites

This guide explains how to build a CLI tool using the `guide` module. Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml`
- Familiarity with `clap` argument parsing

## Adding verbosity and format to your CLI

Embed `CliVerbosity` and `CliFormat` as flattened clap args:

```rust
use clap::Parser;
use genesis::guide::{CliVerbosity, CliFormat, Verbosity, OutputFormat};

#[derive(Parser)]
#[command(name = "my-tool")]
struct Cli {
    #[command(flatten)]
    verbose: CliVerbosity,

    #[command(flatten)]
    format: CliFormat,
}

fn main() {
    let cli = Cli::parse();
    let verbosity = cli.verbose.verbosity();
    let format = cli.format.format();
    // ...
}
```

This gives you:
- `-v` / `-vv` / `-vvv` for progressive verbosity
- `-q` / `--quiet` for silencing output
- `--json` / `--human` for output format, with auto-detection

## Using the `Output` helper

The `Output` type wraps an `Envelope` with a fluent builder API:

```rust
use genesis::guide::{Output, Verbosity};
use std::io::Write;

fn greet(name: &str, verbosity: Verbosity) -> Result<(), Box<dyn std::error::Error>> {
    let output = Output::success(format!("Hello, {name}!"))
        .with_warning("Name contains uppercase characters")
        .with_next_step("Try `my-tool wave` for a wave");

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    output.print(verbosity, &mut stdout, &mut stderr)?;
    Ok(())
}
```

## Format-dispatching with `emit()`

Use `Output::emit()` to let the format decide — human-readable text or JSON envelope:

```rust
use genesis::guide::{Output, OutputFormat, CliFormat};

fn list_items(format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
    let output = Output::success("3 items found")
        .with_data(vec!["item1", "item2", "item3"]);

    output.emit(format, &mut std::io::stdout(), &mut std::io::stderr())?;
    Ok(())
}
```

## Progressive-disclosure verbosity

The verbosity levels control what the user sees. `CliVerbosity` maps the `-v` count
via `Verbosity::from_verbose_count()`:

| Count | `-v` flags | Level | Shows |
| :--- | :--- | :--- | :--- |
| 0 | `-q` / `--quiet` | Quiet | Errors only |
| 1 | _(none)_ | Normal | Result + next step |
| 2 | `-v` | Verbose | + warnings + context |
| 3+ | `-vv` / `-vvv` | Debug | + internals + trace |

Use `Verbosity::help_footer()` to display a hint in your CLI help text:

```rust
fn main() {
    println!("{}", Verbosity::help_footer()); // "Use -v for..."
}
```

## Assembling a CLI with GuideBuilder

`GuideBuilder` (created via `Guide::builder()`) collects all genesis modules into
a coherent CLI runner. It registers valid commands for typo detection, sets up
verbosity, and prepares config integration:

```rust,no_run
use genesis::guide::Guide;

let guide = Guide::builder("my-tool", env!("CARGO_PKG_VERSION"))
    .about("Does something useful")
    .commands(&["init", "doctor", "status"])
    .build();

// Run a command with format-aware output
// guide.run(|g| { ... });
// guide.run_formatted(format, |g| { ... });
```

> See the source code of `wai` or `dont` for complete `Guide` usage examples.

## Error handling with `ErrorSink`

`ErrorSink` wraps errors with self-healing suggestions:

```rust
use genesis::guide::ErrorSink;
use genesis::suggestions::Suggestion;

let mut sink = ErrorSink::new();
sink.add_error("File not found", Some(Suggestion::DidYouMean("config.toml".to_string())));
sink.emit_output(&mut std::io::stderr());
```

## Troubleshooting: Common Fail-States

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| `--json` flag not recognized | `CliFormat` not embedded in args | Add `#[command(flatten)] format: CliFormat` |
| Human output still shows JSON | TTY detection fails in CI | Explicitly pass `--human` or `--json` |
| Warnings not shown | Verbosity too low | Use `-v` or check `Verbosity >= Verbose` |

## Further Exploration

- [Using the Envelope](envelope.md) — detailed envelope patterns
- [Adding a DoctorCheck](doctor.md) — diagnostic framework for your CLI's `doctor` subcommand