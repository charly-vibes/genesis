# genesis-vibes

> A shared Rust crate of cross-cutting CLI/AIX/self-healing infrastructure
> for the charly-vibes tool suite. Each tool depends on it instead of
> reimplementing the same conventions.

genesis-vibes is the shared foundation the suite consistency evaluation
(2026-07-27) and the tool-craft playbook point at. It generalizes
`dont-2j6o` ("extract shared JSON envelope crate") from one module to the
full set of cross-cutting pieces four-of-five tools are missing.

## What it owns (and what it does not)

| Module | Donor | Today present in |
|---|---|---|
| `envelope` | dont | dont only |
| `suggestions` | wai | wai only |
| `managed_block` | wai / dont / espectacular | 3 of 5 |
| `aix` (llms.txt/llm.txt/AGENTS.md generation) | wai | partial |
| `feedback` (redactor + context bundle + error scratch) | new | none |
| `suite_linter` (the `wai doctor --suite` checks) | new (wai-bdqw.8) | none |

**Boundary rule:** if only one tool uses it, it does not belong in the kit.
Domain logic (metrics, stores, engines, analysis) stays in each tool.

## Distribution

[![crates.io](https://img.shields.io/crates/v/genesis-vibes.svg)](https://crates.io/crates/genesis-vibes)

```toml
[dependencies]
genesis-vibes = "0.2"
```

Or use a git dependency for bleeding-edge changes:

```toml
[dependencies]
genesis-vibes = { git = "git@cv:charly-vibes/genesis.git", tag = "v0.2.0" }
```

## Modules

| Module | Status | Description |
|---|---|---|
| `envelope` | stable | Structured CLI output envelope (ported from dont) |
| `suggestions` | stable | Self-healing error suggestions, CommandRegistry (ported from wai) |
| `managed_block` | stable | Managed block injector (ported from wai/dont/espectacular) |
| `aix` | partial | AIX artifact generation (llms.txt/llm.txt/AGENTS.md helpers) |
| `config` | stable | Shared config management via ConfigFile trait + ConfigRegistry |
| `guide` | stable | CLI scaffold: Verbosity, Output, ErrorSink, GuideBuilder |
| `fixture` | stable | Test scratch fixtures and dogfooding runners |
| `feedback` | new | Agent issue reporting: redactor, context bundle, error scratch, gh invocation |
| `suite_linter` | new | Suite-wide config lint checks via LintCheck trait |

## Status

Implemented and published on crates.io (v0.2.0). Downstream adoption is tracked
per-repo via `upgrade-genesis` openspec changes.
