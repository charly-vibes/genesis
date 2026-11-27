> *"Cuando todo era nada*
> *Era nada el principio*
> *Él era el Principio*
> *Y de la noche hizo luz*
> *Y fue el Cielo*
> *Y esto que está aquí"*
> — Vox Dei

# genesis-vibes

> **Why:** every CLI in the charly-vibes suite re-implemented the same
> cross-cutting plumbing — JSON envelopes, verbosity, doctor checks, config
> loading, AGENTS.md blocks — and the implementations drifted apart. genesis
> centralizes it so behavior stays identical across tools and fixes land once.
> [Motivation & design](docs/src/introduction.md)
>
> **Status:** [stable](docs/src/status.md) · shared foundation crate, consumed by all in-org CLIs · crates.io · [charly-vibes Tool Ecosystem](https://charly-vibes.github.io/dulce-de-leche/ecosystem-map.html)
>
> A shared Rust crate of cross-cutting CLI/AIX/self-healing infrastructure
> for the charly-vibes tool suite. Each tool depends on it instead of
> reimplementing the same conventions.

**Questions?** [FAQ & troubleshooting](docs/src/faq.md) · **Status:** [docs/src/status.md](docs/src/status.md)

genesis-vibes is the shared foundation the suite consistency evaluation
(2026-07-27) and the tool-craft playbook point at. It generalizes
`dont-2j6o` ("extract shared JSON envelope crate") from one module to
the full set of cross-cutting pieces the suite needs.

## When to use

- Use genesis when you are building a CLI tool in the charly-vibes suite and
  need the shared plumbing every in-org CLI already agrees on: JSON output
  envelopes, verbosity/format flags, doctor checks, config loading, AGENTS.md
  managed blocks, git hooks, or agent evals.
- Use genesis when you want a behavior fix to land once and reach every suite
  tool through one dependency bump instead of copying the same code again.
- Use genesis when you are wiring a new tool into the ecosystem's
  self-healing contract: suggestions, feedback collection, status dashboards,
  suite-wide lint checks.

**Don't use it when:**

- Your crate is an application with domain logic (metrics, stores, engines,
  analysis) — genesis deliberately owns no domain logic; keep it in your tool.
  **Do not use genesis for domain logic.** It is infrastructure only.
- You need a user-facing CLI binary today — use one of the consumers instead:
  [`dont`](https://github.com/charly-vibes/dont), [`wai`](https://github.com/charly-vibes/wai),
  [`testaruda`](https://github.com/charly-vibes/testaruda).
- You only need a JSON envelope for one private crate with no suite
  interoperability — vendoring ~200 lines beats a cross-tool version contract.

**Assumption genesis makes:** it is a *library dependency* for CLI tools, not
an application — nothing to install or run on its own; your CLI's `main()`
drives it.

## Documentation

Full documentation is available at **[charly-vibes.github.io/genesis/](https://charly-vibes.github.io/genesis/)** —
built with mdBook and deployed to GitHub Pages on every push to `main`.

- [Getting Started](https://charly-vibes.github.io/genesis/getting-started.html) — add genesis-vibes to your project
- [How-to Guides](https://charly-vibes.github.io/genesis/) — envelope, CLI, doctor, fixtures
- [Modules Reference](https://charly-vibes.github.io/genesis/reference/modules.html) — full API reference
- [Architecture & Design](https://charly-vibes.github.io/genesis/explanation/architecture.html) — rationale and trade-offs
- [FAQ & troubleshooting](docs/src/faq.md) · [Status](docs/src/status.md) · [License (Apache-2.0)](LICENSE)

## What it owns (and what it does not)

**Boundary rule:** if only one tool uses it, it does not belong in the kit.
Domain logic (metrics, stores, engines, analysis) stays in each tool.

## Install

[![crates.io](https://img.shields.io/crates/v/genesis-vibes.svg)](https://crates.io/crates/genesis-vibes)

Genesis is a library; the only install path is adding it to your consumer
crate's dependencies:

```toml
[dependencies]
genesis-vibes = "0.15"
```

Or use a git dependency for bleeding-edge changes:

```toml
[dependencies]
genesis-vibes = { git = "git@cv:charly-vibes/genesis.git", tag = "v0.15.0" }
```

## Quick start

Once the dependency compiles (`cargo build` in your crate), the smallest real
use is emitting the suite's shared JSON output envelope — the same one every
charly-vibes CLI prints and every agent parses:

```rust
use genesis_vibes::envelope::{Envelope, EnvelopeKind};

// `cli_version` is YOUR tool's version, not genesis-vibes'.
let env = Envelope::success(
    env!("CARGO_PKG_VERSION"),
    EnvelopeKind::Ok,
    serde_json::json!({ "count": 3 }),
    Vec::new(),
    Vec::new(),
);

println!("{}", serde_json::to_string(&env)?);
assert!(env.ok);
```

Expected result: `cargo build` succeeds and the snippet prints a JSON envelope
like `{"ok":true,"envelope_version":"0.1","cli_version":"<your-tool>","envelope_kind":"Ok",…}`
with exit code 0. Other entry points: [`doctor`](docs/src/how-to/doctor.md) for
diagnostic checks, [`guide`](docs/src/how-to/guide.md) for CLI scaffolding.

## Common workflows

1. **Emit machine-readable command output** — build an `Envelope<T>` with
   `Envelope::success(...)` → your CLI's stdout carries the suite's standard
   `ok`/`data`/`hints` JSON.
2. **Ship a `doctor` subcommand** — implement `DoctorCheck`, run
   `DoctorRunner::new(checks).run(&repo, false)` → a `DoctorReport` with
   pass/warn/fail counts and auto-fix support.
3. **Register your tool for discovery** — call
   `discovery::register(&project_root, name, desc, type, path)` from your
   `init` command → an entry in `.genesis/tools.toml` other suite tools can
   `scan()`.

## Commands

Genesis is a library with no CLI of its own — there is no `--help` to show.
The equivalent table lives in the module list below and the rendered
[Modules Reference](https://charly-vibes.github.io/genesis/reference/modules.html).

## FAQ

- *Do I need this if I already use one of the suite CLIs?* — No; you are
  already consuming it transitively. Genesis is for building new suite tools.
- *What does it write or modify?* — Only what your tool asks it to (managed
  blocks, `.genesis/tools.toml`, git hooks). It never touches files on its own.
- *Is there a binary to install?* — No; it is a lib crate published to
  crates.io. See [status.md](docs/src/status.md) for the recorded deviation.
- More in the [FAQ & troubleshooting page](docs/src/faq.md).

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
| `evals` | new | Agent evals: ErrorTaxonomy, deterministic Scenario replay, eval report contract v1 |
| `git_hooks` | new | Shared git hook primitives: ownership-marked install/uninstall, owner/framework detection, lefthook wiring |

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
- `Output::emit(...)` — format-dispatching output, calls human `print()` or JSON envelope
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

## Development

Repository-development recipes (for contributors to genesis itself, not
consumers):

- Run the CI pipeline: `just ci`
- Refresh the kaya research record: `just record-capture` (skipped on a dirty
  tree)

This repository keeps a kaya research record: `.kaya/ledger.jsonl` captures
observations from the wai tree and recent commits; `kaya build` compiles the
`docs/research/` tree and `kaya publish` writes `docs/public/`. `just
record-capture` refreshes all three (skipped on a dirty tree); the nightly
Record workflow commits the snapshot. Gates: `kaya check` (projections vs
ledger), `scripts/record-lag.py` (the record must not fall more than 60
record-relevant commits behind).

## Documentation provenance

Rendered docs are generated artifacts; the repository sources are
authoritative — edit the repo sources, not the rendered output.
