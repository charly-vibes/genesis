# Using the Envelope

**TL;DR:** Every command returns an `Envelope<T>`. Check `ok` first, then inspect `data`. Use `ErrorResult` for errors — it enforces a non-empty remediation suggestion.

## Context & Prerequisites

This guide explains how to use genesis's structured output envelope for consistent CLI output. Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml`
- Read [Getting Started](../getting-started.md) for basic usage

## Constructing a success envelope

Use `Envelope::success()` (always passing your tool's own CLI version as the
first argument) or the `Output` helper for common cases:

```rust
use genesis::envelope::Envelope;
use genesis::guide::Output;

// Direct envelope construction — cli_version is YOUR tool's version
let env: Envelope<&str> = Envelope::success(
    env!("CARGO_PKG_VERSION"), // your tool, not genesis-vibes
    genesis::envelope::EnvelopeKind::Ok,
    "operation completed",
    vec![], // warnings
    vec![], // hints
);

// Using the Output helper (recommended for CLI commands)
let output = Output::success(vec!["item1", "item2"])
    .with_warning("config file is deprecated, migrate to config.toml");
```

**`cli_version` is caller-supplied.** It identifies the tool that emits the
envelope (e.g. `env!("CARGO_PKG_VERSION")` in *your* crate). Genesis never
injects its own package version — there is no zero-argument constructor. See
the [CLI version ownership contract](../reference/modules.md#cli-version-ownership-contract).

## Adding warnings and hints

Warnings signal non-blocking concerns. Hints suggest next steps.

```rust
let output = Output::success("Project initialized")
    .with_warning("Check your network connection for remote sync")
    .with_next_step("Run `my-tool doctor` to verify setup");
```

## Returning errors

`ErrorResult` enforces **Invariant 3.2.5**: every error must include a remediation suggestion. The constructor returns `Err` if `remediation` is empty.

```rust
use genesis::envelope::{ErrorResult, RemediationEntry};

// Good — remediation is non-empty
let err = ErrorResult::new(
    "E_CONFIG_NOT_FOUND",                      // code
    "config file not found",                   // message
    None,                                      // rule_name
    None,                                      // spec_ref
    None,                                      // entity_id
    vec![],                                    // unmet_clauses
    vec![RemediationEntry {
        command: "my-tool init".to_string(),
        description: "Create a default config".to_string(),
    }],
)?;

// Bad — this returns Err
let err = ErrorResult::new("E_BROKE", "something broke", None, None, None, vec![], vec![]);
// => Err("remediation must be non-empty (Invariant 3.2.5)")
```

Each `RemediationEntry` pairs a runnable `command` with a short `description`.
Consumers render them after the error message — see
[Reading the envelope](#reading-the-envelope) below.

## Choosing the envelope kind

`EnvelopeKind` is a closed enum. Use it to signal the type of response:

| Kind | When to use |
| :--- | :--- |
| `Ok` | Successful operation with data |
| `Error` | Operation failed |
| `Empty` | Successful operation with no data |
| `List` | Returning a collection of items |
| `Check` | Validation or diagnostic result |
| `Doctor` | Doctor run report |
| `Version` | Version information |
| `Stats` | Statistics or metrics |
| `Info` | Informational message |
| `Warning` | Non-blocking concern |

## Reading the envelope

Consumers always check `ok` first:

```rust
use genesis::envelope::ErrorResult;

// A success envelope carries whatever payload the command produced
let envelope: Envelope<Vec<String>> = /* ... */;

if envelope.ok {
    for item in &envelope.data {
        println!("  - {item}");
    }
}
```

An error envelope (`Envelope::error(...)`) carries an `ErrorResult` as its data —
check `ok` first, then render the remediation entries:

```rust
let envelope: Envelope<ErrorResult> = /* ... from a failed command ... */;

if !envelope.ok {
    let err = &envelope.data;
    eprintln!("Error [{}]: {}", err.code, err.message);
    for entry in &err.remediation {
        eprintln!("  → {} — {}", entry.command, entry.description);
    }
}
```

## Troubleshooting: Common Fail-States

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Compile error: `ErrorResult::new` returns `Err` | Empty remediation string | Provide a non-empty remediation suggestion |
| Envelope not printed in JSON format | CLI not using `CliFormat` or `Output::emit()` | Use `Output::emit(cli_version, format, verbosity, ...)` instead of `output.print(...)` |
| Warnings not showing | Verbosity set to `Normal` or `Quiet` | Bump to `Verbose` to see warnings |

## Further Exploration

- [Building a CLI with Guide](guide.md) — progressive-disclosure verbosity levels
- [Adding a DoctorCheck](doctor.md) — diagnostic framework that uses the envelope