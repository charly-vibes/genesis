# Modules Overview

**TL;DR:** Reference table of all genesis-vibes modules with key types, traits, and entry points.

## Module Map

| Module | Status | Key Types / Traits | Entry Point |
| :--- | :--- | :--- | :--- |
| `envelope` | stable | `Envelope<T>`, `EnvelopeKind`, `ErrorResult`, `set_author()` | `Envelope::ok()`, `Envelope::error()` |
| `guide` | stable | `Verbosity`, `Output`, `CliVerbosity`, `CliFormat`, `OutputFormat`, `ErrorSink`, `GuideBuilder`, `Guide` | `Output::success()`, `Output::emit()` |
| `suggestions` | stable | `Suggestion`, `SuggestionEngine`, `CommandRegistry` | `SuggestionEngine::new()` |
| `managed_block` | stable | `BlockDef`, `BlockInjector`, `BlockRegistry` | `BlockInjector::new()` |
| `aix` | stable | AIX artifact generation helpers | `llms.txt`, `llm.txt`, `AGENTS.md` generation |
| `config` | stable | `ConfigFile` trait, `ConfigRegistry`, `ConfigStore` | `ConfigFile::read()` |
| `fixture` | stable | `Fixture`, `FixtureError` | `Fixture::new()` |
| `feedback` | stable | `handle_feedback()`, `FeedbackArgs` | `handle_feedback()` |
| `suite_linter` | stable | `LintCheck` trait, `LinterRegistry`, `LintResult`, `Severity` | `LintCheck::check()` |
| `doctor` | new | `DoctorCheck` trait, `DoctorRunner`, `DoctorReport`, `CheckStatus` | `DoctorRunner::new()` |
| `cli` | new | `generate_completions()`, `maybe_print_version_json()` | `generate_completions()` |
| `status` | new | `StatusContributor` trait, `StatusBuilder`, `StatusLevel`, `StatusSection` | `StatusBuilder::new()` |
| `scaffold` | new | `Scaffold`, `ScaffoldResult` | `Scaffold::new()` |
| `discovery` | new | `scan()`, `register()`, `unregister()`, `Manifest`, `DetectedTool` | `scan()`, `register()` |

---

## envelope

**Signature:** `genesis::envelope`

Structured CLI output envelope. Every command returns an `Envelope<T>`.

### Key Types

| Type | Description |
| :--- | :--- |
| `Envelope<T>` | Generic output envelope with `ok`, `data`, `error`, `warnings`, `hints`, `meta` |
| `EnvelopeKind` | Closed enum: `Ok`, `Error`, `Empty`, `List`, `Check`, `Doctor`, `Version`, `Stats`, `Info`, `Warning` |
| `ErrorResult` | Error with mandatory `remediation` field (constructor returns `Err` if empty) |
| `Meta` | Observability metadata: `duration`, `transaction_id`, `request_id`, `author` |
| `Warning` | Non-blocking concern with message |

### Functions

| Function | Description |
| :--- | :--- |
| `set_author(author: String)` | Set global author for envelope metadata. Call once at startup. |

---

## guide

**Signature:** `genesis::guide`

CLI scaffold: verbosity, output format, error handling, and command dispatch.

### Key Types

| Type | Description |
| :--- | :--- |
| `Verbosity` | Progressive-disclosure enum: `Quiet`, `Normal`, `Verbose`, `Debug` |
| `CliVerbosity` | Embeddable clap args struct for `-v`/`-vv`/`-vvv` + `-q`/`--quiet` |
| `OutputFormat` | `Human` or `Json` |
| `CliFormat` | Embeddable clap args struct for `--json`/`--human` with auto-detection |
| `Output<T>` | Fluent builder wrapping `Envelope<T>` |
| `ErrorSink` | Error collector with self-healing suggestions |
| `GuideBuilder` | Builder for assembling genesis modules into a CLI |
| `Guide` | Complete CLI runner with formatted output |

### Functions

| Function | Description |
| :--- | :--- |
| `Output::success(msg)` | Create a success output |
| `Output::emit(format, stdout, stderr)` | Format-dispatching output (JSON or human) |
| `Verbosity::from_verbose_count(u8)` | Canonical clap count to Verbosity mapping |
| `Verbosity::help_footer()` | "Use -v for..." progressive-disclosure hint |

---

## suggestions

**Signature:** `genesis::suggestions`

Self-healing error suggestions.

### Key Types

| Type | Description |
| :--- | :--- |
| `Suggestion` | `DidYouMean(String)` or `Fix(String)` |
| `SuggestionEngine` | Typo detection engine for subcommands |
| `CommandRegistry` | Registry of known commands for suggestion matching |

---

## managed_block

**Signature:** `genesis::managed_block`

Managed block injector for `<!-- NAME:START -->` / `<!-- NAME:END -->` markers.

### Key Types

| Type | Description |
| :--- | :--- |
| `BlockDef` | Named block with start/end markers |
| `BlockInjector` | Injects content into managed blocks |
| `BlockRegistry` | Collection of `BlockDef` entries |

---

## config

**Signature:** `genesis::config`

Shared config management.

### Key Types

| Type | Description |
| :--- | :--- |
| `ConfigFile` trait | `read()`, `write()`, `validate()` |
| `ConfigRegistry` | Tool registration for config files |
| `ConfigStore` | Discovery and validation of configs |

---

## fixture

**Signature:** `genesis::fixture`

Test scratch environments and runners.

### Key Types

| Type | Description |
| :--- | :--- |
| `Fixture` | Builder for temp directories with markers, files, git init |
| `FixtureError` | `EmptyCommand`, `Spawn`, `Git`, `Io`, `Serde` |

### Fixture builder methods

| Method | Description |
| :--- | :--- |
| `Fixture::new()` | Create a new fixture builder |
| `.with_marker(path)` | Create a marker directory/file |
| `.with_file(path, content)` | Write a file with content |
| `.with_toml(path, value)` | Write a serializable struct as TOML |
| `.with_git_init()` | Initialize a git repo |
| `.build()` | Build the fixture (returns `Fixture`) |
| `.run(program, args)` | Run a command in the fixture directory |

---

## doctor

**Signature:** `genesis::doctor`

Diagnostic framework with auto-fix.

### Key Types

| Type | Description |
| :--- | :--- |
| `DoctorCheck` trait | `name()`, `description()`, `run()`, `can_fix()`, `fix()` |
| `DoctorRunner` | Runs a collection of checks with `run(repo, fix)` |
| `DoctorReport` | Structured result with `summary` and `results` |
| `CheckStatus` | `Pass`, `Warn`, `Fail` |

---

## cli

**Signature:** `genesis::cli`

CLI helpers.

### Functions

| Function | Description |
| :--- | :--- |
| `generate_completions()` | One-liner for clap_complete shell completions |
| `maybe_print_version_json(name, version)` | Pre-parse `--version --json` before clap |

---

## status

**Signature:** `genesis::status`

Cross-tool status dashboard.

### Key Types

| Type | Description |
| :--- | :--- |
| `StatusContributor` trait | `name()`, `status()` |
| `StatusBuilder` | Aggregates all contributors into a unified report |
| `StatusLevel` | `Healthy`, `Warning`, `Error`, `Unknown` |
| `StatusSection` | Named section with items and level |
| `DoctorStatusBridge` | Wraps any `DoctorRunner` as a `StatusContributor` |

---

## scaffold

**Signature:** `genesis::scaffold`

Init scaffolding builder.

### Key Types

| Type | Description |
| :--- | :--- |
| `Scaffold` | Builder for directories, configs, gitignore, managed blocks |
| `ScaffoldResult` | `created: Vec<PathBuf>`, `existed: Vec<PathBuf>` |

### Scaffold builder methods

| Method | Description |
| :--- | :--- |
| `Scaffold::new(path)` | Create a new scaffold for a project path |
| `.dir(dir)` | Create a directory |
| `.default_config(path, content)` | Write a default config file |
| `.gitignore_entry(pattern)` | Add a `.gitignore` entry |
| `.managed_block(name, content)` | Inject a managed block |
| `.agent_command_file(path, content)` | Create an agent command file |
| `.build()` | Build the scaffold (returns `ScaffoldResult`) |

---

## discovery

**Signature:** `genesis::discovery`

Tool discovery via `.genesis/tools.toml` manifest.

### Key Types

| Type | Description |
| :--- | :--- |
| `Manifest` | TOML manifest with `tools` table |
| `DetectedTool` | `name`, `description`, `detected` (bool), `detector_type` |

### Functions

| Function | Description |
| :--- | :--- |
| `scan(project)` | Scan for all registered tools |
| `register(project, name, desc, type, path)` | Register a tool |
| `unregister(project, name)` | Remove a tool registration |
| `list_tools(project)` | List all registered tools |
| `has_manifest(project)` | Check if manifest exists |