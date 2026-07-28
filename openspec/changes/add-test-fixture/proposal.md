# Change: Add `test` module (scratch fixtures for testing and dogfooding)

## Why

Every tool in the suite needs test fixtures: temp directories with configs,
markers, and project files. Currently each tool creates them ad-hoc:

```
wai:         tempfile::tempdir() + manual dir creation in tests
dont:        tempfile::tempdir() + manual config writes
testaruda:   tempfile::tempdir() + manual adapters setup
pretender:   (no tests yet)
espectacular: tempfile::tempdir() + manual config
```

A new tool (crua, livin, vampiro) has to write the same boilerplate. This
is also the dogfooding pattern — running the tool against itself in a
scratch environment to verify CLI behavior end-to-end.

A `genesis::fixture` module provides reusable fixtures so every tool writes
zero lines of scratch setup — just `Fixture::new().with_config().with_marker().build()`.

## What Changes

### `Fixture` — scratch directory builder

```rust
use genesis::fixture::Fixture;

let fixture = Fixture::new()
    .with_marker(".wai")                          // creates .wai/ dir
    .with_marker(".dont")                         // creates .dont/ dir
    .with_marker_dir(".beads/hooks")              // creates nested dir
    .with_config(cfg)                              // writes config via ConfigFile trait
    .with_toml("testaruda.toml", toml_value)      // writes any TOML file
    .with_file("src/main.rs", "fn main() {}")     // writes any file
    .with_git_init()                              // git init + initial commit (Result)
    .build();

// fixture.path() -> &Path (root of the scratch dir)
// fixture.path(".wai/config.toml") -> PathBuf
// fixture.cleanup() — called on Drop
```

`with_git_init()` returns a `Result` — if `git` is not installed, it fails
gracefully rather than panicking. Tests that need git should use
`fixture.with_git_init().expect("git not available")` or skip-if-unavailable.

### `Fixture::run()` — dogfooding

```rust
let output = fixture.run(&["my-tool", "check", "--json"])?;
// output.stdout: String
// output.stderr: String
// output.exit_code: i32
// output.json::<T>() -> Option<T>  — parse JSON from stdout
```

Runs the tool as a subprocess in the fixture directory. Uses argv array
(not a string) to handle quoting and spaces correctly. This is the
dogfooding pattern — the tool tests itself against a known environment.

### `Fixture::assert_*` — test helpers

```rust
fixture.assert_file_exists(".wai/config.toml");
fixture.assert_file_contains("llm.txt", "genesis");
fixture.assert_marker(".beads");
```

### `ConfigFixture` — typed config setup

```rust
// Uses the ConfigFile trait from genesis::config
let cfg = MyConfig { threshold: 0.5, ..default() };
Fixture::new()
    .with_config(cfg)  // tool name comes from ConfigFile::TOOL_NAME
    .build();
```

This works because `MyConfig` implements `ConfigFile`, which provides
`write()` and `TOOL_NAME`. If the tool hasn't adopted `genesis::config`
yet, use `.with_toml()` instead.

### What moves to genesis

- `Fixture` struct with builder pattern (markers, configs, files, git init)
- `Fixture::run(args: &[&str])` — subprocess execution in fixture dir
- `Fixture::assert_*` — test assertions
- Integration with `genesis::config::ConfigFile` for typed config setup
- `FixturePath` — typesafe path resolution within fixture

### What stays in each tool

- Domain-specific test logic (e.g. testaruda's adapter tests)
- Test assertions about domain behavior (not infrastructure)

## Impact

- **New capability**: `fixture` — scratch fixtures for testing and dogfooding.
- **Affected code**: new `src/fixture/` module in genesis. Downstream tools
  replace ad-hoc `tempfile::tempdir()` calls with `Fixture::new()`.
- **Blocked by**: genesis v0.1.0 already tagged (test can be v0.2.0).
- **Migration path**: additive — tools can adopt one fixture at a time.
  Old ad-hoc tempdir code can coexist during migration.