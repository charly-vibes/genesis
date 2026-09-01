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
genesis-vibes = "0.6"
```

If you want the bleeding-edge version from the repository instead of the crates.io release:

```toml
[dependencies]
genesis-vibes = { git = "git@cv:charly-vibes/genesis.git", tag = "v0.6.0" }
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

Run your tool — you will see the result on stdout and a next-step hint on stderr:

```text
$ my-tool init
"Project initialized"
→ Run: Run `my-tool doctor` to verify setup
```

The first line is the `data` payload (rendered with `Debug` formatting, hence the
quotes); the `→ Run:` line is the next-step hint, written to stderr so it stays
out of piped output.

## Step 3: Add `--json` output with format auto-detection

Embed `CliFormat` into your clap args to get automatic TTY detection:

```rust
use clap::Parser;
use genesis::guide::{CliFormat, CliVerbosity, Output, OutputFormat, Verbosity};

#[derive(Parser)]
#[command(name = "my-tool")]
struct Cli {
    #[command(flatten)]
    verbose: CliVerbosity,
    #[command(flatten)]
    format: CliFormat,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let format: OutputFormat = cli.format.format();
    let verbosity: Verbosity = cli.verbose.verbosity();

    let output = Output::success(vec!["item1", "item2"]);
    output.emit(
        env!("CARGO_PKG_VERSION"), // your tool's version, not genesis's
        format,
        verbosity,
        &mut std::io::stdout(),
        &mut std::io::stderr(),
    )?;
    Ok(())
}
```

> `emit()` requires the data payload to implement `Serialize` (needed for the
> JSON branch). Passing `cli_version` is your responsibility — see the
> [cli-version ownership contract](reference/modules.md#cli-version-ownership-contract).

This auto-detects:
- **TTY** (interactive terminal) → human-readable text
- **Piped/redirected** (`|`, `>`, CI) → JSON envelope

Either can be overridden with `--json` or `--human`.

## Step 4: Print version as JSON

Pre-parse `--version --json` before clap processes the rest of the args. The function
returns `true` if it printed the version envelope — and in that case **you** must
exit (it does not call `std::process::exit()` for you):

```rust
use genesis::cli::maybe_print_version_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // If `--version --json` (or `--version -j`) is passed, this prints the version
    // envelope and returns `true` — exit so clap doesn't handle --version too.
    // Plain `--version` is left for clap; the function returns `false`.
    if maybe_print_version_json("my-tool", env!("CARGO_PKG_VERSION")) {
        return Ok(());
    }

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