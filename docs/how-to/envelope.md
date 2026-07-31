# Using the Envelope

**TL;DR:** Every command returns an `Envelope<T>`. Check `ok` first, then inspect `data`. Use `ErrorResult` for errors — it enforces a non-empty remediation suggestion.

## Context & Prerequisites

This guide explains how to use genesis's structured output envelope for consistent CLI output. Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml`
- Read [Getting Started](../getting-started.md) for basic usage

## Constructing a success envelope

Use `Envelope::ok()` or the `Output` helper for common cases:

```rust
use genesis::envelope::Envelope;
use genesis::guide::Output;

// Direct envelope construction
let env: Envelope<&str> = Envelope::ok("operation completed", None);

// Using the Output helper (recommended for CLI commands)
let output = Output::success("done")
    .with_data(vec!["item1", "item2"])
    .with_warning("config file is deprecated, migrate to config.toml");
```

## Adding warnings and hints

Warnings signal non-blocking concerns. Hints suggest next steps.

```rust
let output = Output::success("Project initialized")
    .with_warning("Check your network connection for remote sync")
    .with_next_step("Run `my-tool doctor` to verify setup");
```

## Returning errors

`ErrorResult` enforces **Invariant 3.2.5**: every error must include a remediation suggestion. The constructor returns `Err` if remediation is empty.

```rust
use genesis::envelope::ErrorResult;

// Good — remediation is non-empty
let err = ErrorResult::new(
    "config file not found",
    "Run `my-tool init` to create a default config",
)?;

// Bad — this returns Err(EmptyRemediation)
let err = ErrorResult::new("something broke", "");
// => ErrorResult::new("something broke", "") returns Err(...)
```

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
let envelope: Envelope<Vec<String>> = /* ... */;

if envelope.ok {
    if let Some(items) = envelope.data {
        for item in items {
            println!("  - {item}");
        }
    }
} else {
    eprintln!("Error: {}", envelope.error.unwrap().message);
    eprintln!("Fix: {}", envelope.error.unwrap().remediation);
}
```

## Troubleshooting: Common Fail-States

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Compile error: `ErrorResult::new` returns `Err` | Empty remediation string | Provide a non-empty remediation suggestion |
| Envelope not printed in JSON format | CLI not using `CliFormat` or `Output::emit()` | Use `Output::emit(format, ...)` instead of `output.print(...)` |
| Warnings not showing | Verbosity set to `Normal` or `Quiet` | Bump to `Verbose` to see warnings |

## Further Exploration

- [Building a CLI with Guide](guide.md) — progressive-disclosure verbosity levels
- [Adding a DoctorCheck](doctor.md) — diagnostic framework that uses the envelope