# Getting Started with genesis-vibes

**TL;DR:** Add `genesis-vibes` as a dependency, import the modules you need, and use `Output::success()` for structured CLI output.

## What You Will Learn

By the end of this tutorial you will have added genesis-vibes to a Rust tool project, emitted structured CLI output with the envelope, and printed version info in machine-readable JSON. No prior knowledge of genesis-vibes is assumed.

## Prerequisites

- A Rust project that uses `clap` for CLI argument parsing
- Rust 1.75 or later

## Step 1: Add the dependency

Add genesis-vibes to your `Cargo.toml`:

```toml
[dependencies]
genesis-vibes = "0.4"
```

If you want the bleeding-edge version from the repository instead of the crates.io release:

```toml
[dependencies]
genesis-vibes = { git = "git@cv:charly-vibes/genesis.git", tag = "v0.4.0" }
```

## Step 2: Emit structured output

Replace raw `println!()` calls with genesis's `Output` type. Every command returns an `Envelope<T>` — callers check `ok` first.

```rust
use genesis::guide::{Output, Verbosity};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Output::success("Project initialized")
        .with_next_step("Run `my-tool doctor` to verify setup");

    let verbosity = Verbosity::Normal;
    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();

    output.print(verbosity, &mut stdout, &mut stderr)?;
    Ok(())
}
```

Run your tool — you will see a clean, single-line result with a next-step hint.

## Step 3: Add `--json` output with format auto-detection

Embed `CliFormat` into your clap args to get automatic TTY detection:

```rust
use clap::Parser;
use genesis::guide::{CliFormat, Output};

#[derive(Parser)]
#[command(name = "my-tool")]
struct Cli {
    #[command(flatten)]
    format: CliFormat,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let format: genesis::guide::OutputFormat = cli.format.format();

    let output = Output::success("done").with_data(vec!["item1", "item2"]);
    output.emit(format, &mut std::io::stdout(), &mut std::io::stderr())?;
    Ok(())
}
```

This auto-detects:
- **TTY** (interactive terminal) → human-readable text
- **Piped/redirected** (`|`, `>`, CI) → JSON envelope

Either can be overridden with `--json` or `--human`.

## Step 4: Print version as JSON

Pre-parse `--version --json` before clap processes the rest of the args. The function
calls `std::process::exit()` internally if the flag is matched, so execution never
continues to the clap setup:

```rust
use genesis::cli::maybe_print_version_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If `--version --json` is passed, this prints the version envelope and exits.
    // If not, it returns immediately and execution continues.
    maybe_print_version_json("my-tool", env!("CARGO_PKG_VERSION"));

    // ... rest of your clap setup
    Ok(())
}
```

> **Note:** The examples in Step 3 require `clap` with the `derive` feature in your
> `Cargo.toml`:
> ```toml
> [dependencies]
> clap = { version = "4", features = ["derive"] }
> ```

## Recap & Next Steps

You added genesis-vibes to your project, emitted structured CLI output, enabled TTY-aware format detection, and wired up `--version --json`. Next, explore:

- [Using the Envelope](how-to/envelope.md) — deeper dive into warning/hint/error patterns
- [Building a CLI with Guide](how-to/guide.md) — verbosity levels, error sinks, suggestions
- [Adding a DoctorCheck](how-to/doctor.md) — diagnostic checks with auto-fix