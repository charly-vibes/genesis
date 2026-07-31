# Writing Tests with Fixture

**TL;DR:** Use `Fixture::new()` to create temporary scratch directories with markers, config files, and git initialization for integration tests.

## Context & Prerequisites

This guide explains how to use the `fixture` module for test scratch environments. Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml` (dev-dependencies is fine)
- A tool that operates on a project directory

## Creating a basic fixture

Build a temporary directory with markers and files:

```rust
use genesis::fixture::Fixture;

#[test]
fn test_detects_marker() {
    let fixture = Fixture::new()
        .with_marker(".my-tool")
        .with_file("config.toml", "key = \"value\"\n")
        .build()
        .expect("build fixture");

    // The fixture path is a real temp directory
    assert!(fixture.path(".my-tool").exists());
    assert!(fixture.path("config.toml").exists());

    // Contents match
    let contents = std::fs::read_to_string(fixture.path("config.toml")).unwrap();
    assert_eq!(contents, "key = \"value\"\n");
}
```

## Adding git initialization

Use `with_git_init()` to set up a git repo for commands that detect git state:

```rust
let fixture = Fixture::new()
    .with_marker(".wai")
    .with_git_init()
    .build()
    .expect("build fixture");

assert!(fixture.path(".git").exists());
```

## Running commands inside the fixture

Use `Fixture::run()` to execute a command with the fixture as the working directory:

```rust
#[test]
fn test_tool_init() {
    let fixture = Fixture::new()
        .build()
        .expect("build fixture");

    let output = fixture.run("my-tool", &["init"])
        .expect("run my-tool init");

    assert!(output.status.success());
    assert!(fixture.path(".my-tool/config.toml").exists());
}
```

## Writing TOML configs

Use `with_toml()` to write a serializable struct as a TOML file:

```rust
use serde::Serialize;

#[derive(Serialize)]
struct ToolConfig {
    name: String,
    enabled: bool,
}

#[test]
fn test_reads_config() {
    let fixture = Fixture::new()
        .with_toml("my-tool.toml", &ToolConfig {
            name: "test".into(),
            enabled: true,
        })
        .build()
        .expect("build fixture");

    assert!(fixture.path("my-tool.toml").exists());
}
```

## Using assertions

`Fixture` provides convenience assertion methods:

```rust
let fixture = Fixture::new()
    .with_marker(".wai")
    .with_file("data.txt", "hello")
    .build()?;

// Check that the fixture has the expected structure
assert!(fixture.path(".wai").exists());
assert!(fixture.path("data.txt").exists());
```

## Troubleshooting: Common Fail-States

| Symptom | Cause | Fix |
| :--- | :--- | :--- |
| Fixture build fails with "EmptyCommand" | `Fixture::run()` called with empty program | Pass a program name as the first argument |
| `with_git_init()` fails | `git` not installed in test environment | Install git or use `Fixture::new()` without git init |
| Temp directory not cleaned up after test | Test panics before Fixture is dropped | Use `Fixture` in a test that doesn't panic, or wrap in `std::panic::catch_unwind` |

## Further Exploration

- [Module Reference](../reference/modules.md) — full `Fixture` API