# Design: add-evals-guidelines — genesis-level decisions

Recorded from the `add-evals-guidelines` openspec change (2026-09-20).

## Why per-consumer harnesses (design D2)

The live-runner implementation stays **out of genesis** deliberately:

- genesis owns the deterministic eval core (`Scenario`, `ErrorTaxonomy`,
  distractor checks, report serialization) — the machinery that must not
  fork. Each tool repo owns its eval battery *and its harness*, implementing
  the shared conventions (action protocol, report contract).
- A suite-wide harness would put domain logic in genesis, violating the
  boundary rule, and would couple every tool's release cadence to one
  runner's model-provider choices.
- The interoperability that matters — comparable fault attribution and
  aggregatable report rows — is carried by the shared taxonomy
  (`ERR_*` codes) and the versioned report contract, not by shared
  machinery.
- Escape hatch: if ≥ 2 tools converge on the same harness code, extracting
  it *then* satisfies the boundary rule — not before.

## Why spec-as-source-of-truth (design D1, D4)

- The guidelines live as a spec capability (`evals-guidelines`) plus book
  docs — not as a module. Rules that live only in prose drift; rules in
  `openspec/specs/` are versioned, scenario-tested, and referenced by the
  book (each how-to section names the requirement it elaborates).
- The v1 report contract is normative at the **docs/fixture level**: the
  reference page (`docs/reference/eval-report.md`) plus two checked-in
  fixtures (`tests/golden/eval_report_*.json`) are the source of truth.
  `ScenarioReport` keeps its replay-tier shape until the first consumer
  harness lands; it then evolves additively (or bumps the version). This
  keeps machinery out of genesis while making the contract concrete enough
  to diff against.
- Genesis validates the guidelines only by making the shared vocabulary
  (`ErrorTaxonomy::ActionFormatViolation`) and the report contract concrete;
  adoption is enforced by the advisory `EvalsGuidelinesAdoption` linter
  probe, not by machinery.

## What ships where

| Deliverable | Home |
|---|---|
| Normative rules | `openspec` spec capability `evals-guidelines` |
| Shared vocabulary | `ErrorTaxonomy::ActionFormatViolation` in `src/evals.rs` |
| Report contract | `docs/reference/eval-report.md` + `tests/golden/eval_report_*.json` |
| Adoption probe | `EvalsGuidelinesAdoption` in `src/suite_linter.rs` |
| Per-repo batteries | 5 drafted tickets (dont, wai, espectacular, pretender, testaruda) — tier 2 deferred |
