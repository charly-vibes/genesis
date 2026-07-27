# Project Context

## Purpose

genesis is a shared Rust crate of cross-cutting CLI, AIX, and self-healing
infrastructure for the charly-vibes suite. It generalizes the in-flight
`dont-2j6o` effort ("extract shared JSON envelope crate") from one module to
the full set of pieces the suite consistency evaluation flagged as missing
across four-of-five tools.

The authoritative proposal is in `openspec/changes/add-genesis-foundation/`.
Implementation is blocked until that proposal is approved.

## Boundary rule (normative)

A capability belongs in genesis **only if two or more suite tools need it**.
If only one tool uses it, it stays in that tool. Domain logic (pretender
metrics, dont's cozo store, espectacular's scenario contracts, testaruda's
Datalog engine, wai's PARA state, the trio's static analysis) is **never** in
the genesis crate.

## Tech Stack

- Implementation language: Rust, stable toolchain, rustfmt + Clippy clean.
- Distributed as a git dependency from `github.com/charly-vibes/genesis`
  (no crates.io publish until the interface stabilizes — per `dont-2j6o`).
- SemVer; breaking changes require a deprecation cycle and bumping all
  dependents in a coordinated change.

## Modules

- `envelope` — shared JSON envelope (`dont-2j6o` folded in; dont is donor).
- `suggestions` — `Suggestion` enum + `suggest_typo` + `WrongOrder` (wai donor).
- `managed_block` — managed-block injector + block format (wai/dont/espectacular donors).
- `aix` — `llms.txt` / `llm.txt` / `AGENTS.md` block generation.
- `feedback` — redactor + context-bundle serializer + error scratch (new).
- `suite_linter` — the `wai doctor --suite` checks (`wai-bdqw.8`).
