# Adding a DoctorCheck

**TL;DR:** Implement the `DoctorCheck` trait and register it with `DoctorRunner` to add structured diagnostics with optional auto-fix to your tool.

## Context & Prerequisites

This guide explains how to add diagnostic checks to your tool's `doctor` subcommand. Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml`
- A working CLI (see [Building a CLI with Guide](guide.md))

## Implementing a basic check

Create a struct that implements `DoctorCheck`:

```rust
use genesis::doctor::{DoctorCheck, DoctorRunner};
use genesis::suite_linter::{LintResult, Severity};
use std::path::Path;

struct ConfigFileCheck;

impl DoctorCheck for ConfigFileCheck {
    fn name(&self) -> &'static str {
        "config-file"
    }

    fn description(&self) -> &'static str {
        "Checks that the tool config file exists and is valid"
    }

    fn run(&self, repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let config_path = repo.join("my-tool.toml");

        if !config_path.exists() {
            return Ok(vec![LintResult::error(
                "config-file",
                "Config file not found",
                "Run `my-tool init` to create a default config",
            )]);
        }

        Ok(vec![]) // pass — no issues
    }
}
```

## Adding auto-fix

Implement `fix` to provide automatic remediation:

```rust
impl DoctorCheck for ConfigFileCheck {
    // ... name, description, run as above ...

    fn can_fix(&self) -> bool {
        true
    }

    fn fix(&self, repo: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let config_path = repo.join("my-tool.toml");

        if config_path.exists() {
            return Ok(vec![]); // already fixed
        }

        std::fs::write(&config_path, "# Default config\nkey = \"value\"\n")?;

        Ok(vec![LintResult::info(
            "config-file",
            "Created default config file",
        )])
    }
}
```

## Running checks with DoctorRunner

Register checks and run them:

```rust
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = DoctorRunner::new(vec![
        Box::new(ConfigFileCheck),
    ]);

    let repo = std::env::current_dir()?;
    let report = runner.run(&repo, false)?; // false = don't fix

    println!("pass={} warn={} fail={}",
        report.summary.pass,
        report.summary.warn,
        report.summary.fail,
    );

    Ok(())
}
```

Pass `true` to enable auto-fix:

```rust
let report = runner.run(&repo, true)?; // true = run fixes
```

## Integrating with the envelope

`DoctorReport` is serializable, so you can emit it as a JSON envelope:

```rust
use genesis::doctor::DoctorReport;
use genesis::envelope::Envelope;

let report: DoctorReport = /* ... */;
let envelope = Envelope::from(report);
envelope.print(&mut std::io::stdout())?;
```

## Panic isolation

If a `DoctorCheck::run()` panics, the runner propagates the panic — it does not
catch it. Wrap checks in `std::panic::catch_unwind` if you need panic isolation
for individual checks.

## Troubleshooting: Common Fail-States

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| `fix()` never called | `run()` called with `false` | Pass `true` as the second argument |
| LintResult not showing in report | Wrong severity level | Use `LintResult::error()` for failures, `LintResult::warn()` for warnings, `LintResult::info()` for informational |
| DoctorCheck panics on missing repo path | Path doesn't exist | Check `repo.exists()` before running checks |

## Further Exploration

- [Using the Envelope](envelope.md) — structured output for doctor reports
- [Module Reference](../reference/modules.md) — full `DoctorCheck` trait API