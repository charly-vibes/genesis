# genesis

> Status: **Proposal** — a shared Rust crate of cross-cutting CLI/AIX/self-healing
> infrastructure for the charly-vibes suite. Each tool depends on it instead of
> reimplementing the same conventions.

genesis is the shared foundation the suite consistency evaluation
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

Git dependency from `github.com/charly-vibes/genesis`, no crates.io publish
until the interface stabilizes (per `dont-2j6o`'s decision).

## Status

Specification and proposal stage — no `src/` yet. Implementation is blocked
until the foundation proposal (`openspec/changes/add-genesis-foundation`) is
approved.
