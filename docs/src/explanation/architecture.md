# Architecture

**TL;DR:** genesis-vibes is a shared crate that generalizes cross-cutting concerns (CLI output, config, diagnostics, scaffolding) from the charly-vibes tool suite into reusable abstractions.

## Why a shared crate?

Before genesis, each tool in the charly-vibes suite reimplemented the same patterns: structured JSON output, verbosity levels, config file management, test fixtures, and agent feedback. Each implementation was identical in intent but diverged in details — a classic case of structural duplication.

genesis-vibes extracts these patterns into a single crate that every tool depends on. This ensures:

- **Consistent CLI output** — every tool speaks the same envelope protocol
- **Single source of truth** — a bug fix in the envelope propagates to all tools
- **Lower maintenance** — new modules (doctor, status, discovery) ship once and are available everywhere

## Module boundaries

Modules are organized by cross-cutting concern. The boundary rule: **if only one tool uses it, it does not belong in genesis.**

```
┌─────────────────────────────────────────────────────────┐
│                    genesis-vibes                         │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ envelope │  │  guide   │  │  config  │  ...          │
│  │ (output) │  │  (CLI)   │  │  (files) │              │
│  └──────────┘  └──────────┘  └──────────┘              │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │  doctor  │  │  status  │  │ scaffold │              │
│  │ (checks) │  │(dashboard)│  │ (init)   │              │
│  └──────────┘  └──────────┘  └──────────┘              │
│                                                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐              │
│  │ fixture  │  │ feedback │  │ discovery│              │
│  │ (tests)  │  │ (issues) │  │ (manifest)│              │
│  └──────────┘  └──────────┘  └──────────┘              │
└─────────────────────────────────────────────────────────┘
         ▲            ▲            ▲
         │            │            │
    ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
    │  wai    │  │  dont   │  │testaruda│  ...
    └─────────┘  └─────────┘  └─────────┘
```

## Module relationships

Some modules build on others:

- **guide** uses **envelope** — `Output<T>` wraps `Envelope<T>`
- **doctor** uses **suite_linter** — `DoctorCheck` produces `LintResult`
- **status** uses **doctor** — `DoctorStatusBridge` wraps `DoctorRunner` as a `StatusContributor`
- **feedback** uses **envelope** — error reporting follows the envelope protocol
- **scaffold** uses **managed_block** — `managed_block()` delegates to `BlockInjector`
- **discovery** uses **config** — `Manifest` is a `ConfigFile`

## Extending genesis

New modules should follow the same patterns:

1. Define a **trait** for the abstraction (e.g., `DoctorCheck`, `StatusContributor`)
2. Provide a **runner/builder** that orchestrates implementations
3. Implement **envelope serialization** for structured output
4. Document the module's **boundary rule** — what belongs in genesis vs. what stays in the tool

## Agent workflow integration

genesis modules are designed to be consumed by AI agents as well as humans:

- **Envelope** — agents parse JSON output; `ok` field is a binary success check
- **Feedback** — agents can call `handle_feedback()` to report issues
- **Discovery** — agents scan `.genesis/tools.toml` to discover registered tools
- **Scaffold** — agents use `init` commands to set up project structure
- **AIX** — agents generate `llms.txt`/`llm.txt` files for project context