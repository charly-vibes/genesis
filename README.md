> *"Cuando todo era nada*
> *Era nada el principio*
> *Él era el Principio*
> *Y de la noche hizo luz*
> *Y fue el Cielo*
> *Y esto que está aquí"*
> — Vox Dei

# genesis-vibes

> A shared Rust crate of cross-cutting CLI/AIX/self-healing infrastructure
> for the charly-vibes tool suite. Each tool depends on it instead of
> reimplementing the same conventions.

genesis-vibes is the shared foundation the suite consistency evaluation
(2026-07-27) and the tool-craft playbook point at. It generalizes
`dont-2j6o` ("extract shared JSON envelope crate") from one module to
the full set of cross-cutting pieces the suite needs.

## What it owns (and what it does not)

**Boundary rule:** if only one tool uses it, it does not belong in the kit.
Domain logic (metrics, stores, engines, analysis) stays in each tool.

## Distribution

[![crates.io](https://img.shields.io/crates/v/genesis-vibes.svg)](https://crates.io/crates/genesis-vibes)

```toml
[dependencies]
genesis-vibes = "0.4"
```

Or use a git dependency for bleeding-edge changes:

```toml
[dependencies]
genesis-vibes = { git = "git@cv:charly-vibes/genesis.git", tag = "v0.4.0" }
```

## Modules

| Module | Status | Description |
|---|---|---|
| `envelope` | stable | Structured CLI output envelope (ported from dont) |
| `suggestions` | stable | Self-healing error suggestions, CommandRegistry (ported from wai) |
| `managed_block` | stable | Managed block injector (ported from wai/dont/espectacular) |
| `aix` | stable | AIX artifact generation (llms.txt/llm.txt/AGENTS.md helpers) |
| `config` | stable | Shared config management via ConfigFile trait + ConfigRegistry |
| `guide` | stable | CLI scaffold: Verbosity, CliVerbosity, OutputFormat, CliFormat, Output, ErrorSink, GuideBuilder, Guide |
| `fixture` | stable | Test scratch fixtures and dogfooding runners |
| `feedback` | stable | Agent issue reporting: redactor, context bundle, error scratch, gh invocation |
| `suite_linter` | stable | Suite-wide config lint checks via LintCheck trait |
| `doctor` | new | Diagnostic framework: DoctorCheck trait, DoctorRunner, DoctorReport with auto-fix |
| `cli` | new | CLI helpers: generate_completions, maybe_print_version_json |
| `status` | new | Cross-tool status/prime dashboard: StatusContributor trait, StatusBuilder |
| `scaffold` | new | Init scaffolding: Scaffold builder for dirs, configs, gitignore, managed blocks |
| `discovery` | new | Tool discovery via .genesis/tools.toml manifest: scan, register, unregister |

## Module details

### `envelope`
Structured JSON output with `ok`, `envelope_version`, `envelope_kind`, `data`, `warnings`, `hints`, `meta`.
Every command returns `Envelope<T>`. Callers check `ok` first.

### `suggestions`
Typo detection via `SuggestionEngine` + `CommandRegistry`. Suggests `DidYouMean` and `Fix` corrections
when a user types an unknown subcommand.

### `managed_block`
Injector for `<!-- BLOCK:START -->` / `<!-- BLOCK:END -->` managed blocks in AGENTS.md and other
markdown files. Used by wai, dont, testaruda, espectacular, vampiro.

### `aix`
Helpers for generating `llms.txt`, `llm.txt`, and `AGENTS.md` agent blocks.

### `config`
`ConfigFile` trait with `read()`/`write()`/`validate()`, `ConfigRegistry` for tool registration,
`ConfigStore` for discovery and validation.

### `guide`
`Guide` builder with `Guide::run()` and `Guide::run_formatted()` for command dispatch,
`ErrorSink` for self-healing error output, `Output<T>` for structured CLI output,
`Verbosity` for progressive disclosure.

**v0.4.0 additions:**
- `CliVerbosity` — embeddable clap args struct for `-v`/`-vv`/`-vvv` + `-q`/`--quiet`
- `CliFormat` — embeddable clap args struct for `--json`/`--human`
- `OutputFormat` enum (`Human` | `Json`)
- `Output::emit(format, ...)` — format-dispatching output, calls human `print()` or JSON envelope
- `Verbosity::from_verbose_count(u8)` — canonical clap count to Verbosity mapping
- `Verbosity::help_footer()` — "Use -v for..." progressive-disclosure hint

The key design: **JSON is the default for non-TTY contexts.** When neither `--json` nor
`--human` is specified, `CliFormat::format()` auto-detects:
- stdout is a **TTY** (interactive shell) → `Human` (readable, colored output)
- stdout is **piped/redirected** (agent, CI, `|`, `> file`) → `Json` (machine-readable `Envelope`)

This means agents and CI pipelines always get parseable JSON without any flags,
while a human at a terminal gets readable output. Either can be overridden explicitly
with `--json` or `--human`.

### `fixture`
`Fixture` builder with `with_file()`, `with_toml()`, `with_marker()`, `with_git_init()`, and
`Fixture::run()` for dogfooding commands in test environments.

### `feedback`
`handle_feedback()` function that any tool calls from its `feedback` subcommand. Wraps scratch
(error persistence), context (env bundle), redactor (privacy), and gh (GitHub issue creation).

### `suite_linter`
`LintCheck` trait + `LinterRegistry` for suite-wide config lint checks. Used by testaruda's doctor.

### `doctor`
`DoctorCheck` trait (with optional auto-fix), `DoctorRunner` (orchestrates checks with fix-verify cycle),
`DoctorReport` (structured results with JSON envelope). Replaces each tool's custom CheckResult/DoctorPayload.

### `cli`
`generate_completions()` — one-liner for `clap_complete` shell completions.
`maybe_print_version_json()` — pre-parse `--version --json` before clap processes args.

### `status`
`StatusContributor` trait for tools to register health state. `StatusBuilder` aggregates all
contributors. `DoctorStatusBridge` wraps any `DoctorRunner` as a contributor automatically.

### `scaffold`
`Scaffold` builder for standardizing `init` commands: `.dir()`, `.default_config()`, `.gitignore_entry()`,
`.managed_block()`, `.agent_command_file()`. Returns `ScaffoldResult` with created/existed paths.

### `discovery`
Tool discovery via `.genesis/tools.toml` manifest. Genesis-based tools self-declare during `init`
so orchestration tools like wai discover them without hardcoding.
- `scan(project)` → `Vec<DetectedTool>`
- `register(project, name, desc, type, path)` — add/update entry
- `unregister(project, name)` — remove entry
- `list_tools(project)`, `has_manifest(project)`

## Status

Published on crates.io (v0.3.0). All 6 Rust tools in the suite depend on genesis.