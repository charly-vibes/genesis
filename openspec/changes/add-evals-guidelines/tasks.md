# Tasks: add-evals-guidelines

## 1. Spec capability ratified
- [x] 1.1 Review and approve the `evals-guidelines` delta (this change) — the rules below implement it verbatim.

## 2. Shared vocabulary (code)
- [x] 2.1 Add `ErrorTaxonomy::ActionFormatViolation` with code `ERR_ACTION_FORMAT_VIOLATION` and one-line agent-actionable description.
- [x] 2.2 Round-trip tests for the new code; guard existing codes unchanged.
- [x] 2.3 `with_model` doc update: verbatim ids including `:free` suffix (ties to the MODIFIED evals requirement).

## 3. Report contract
- [x] 3.1 Define the `report_version` 1 JSON shape (required/optional fields per the interoperable-report requirement) as the Contract section of `docs/reference/eval-report.md` plus the fixtures in 3.2 — docs/fixture-normative; `ScenarioReport` keeps its replay-tier shape until the first consumer harness lands (additive evolution then).
- [x] 3.2 Publish two example report fixtures (JSON) checked into the repo for consumers to diff against: one tier-2 row (model id + repetition present), one non-tier-2 row (model id omitted); absent optional fields omitted, never `null`.
- [x] 3.3 Validate the non-tier-2 fixture round-trips through serde JSON serialization of `ScenarioReport`; document the delta for live-only fields (`report_version`, `status`, repetition index, bounds) against the current `ScenarioReport` shape.

## 4. mdBook — explanation and how-to
- [x] 4.1 Extend `docs/how-to/evals.md` with the live cadence: tier ladder table (claims per tier), action protocol, bounds, and battery maintenance (staleness review).
- [x] 4.2 New how-to: `docs/how-to/evals-ci.md` — running the ladder in CI: static on push, nightly replay, rotated `:free` matrix; 429 semantics; budget.
- [x] 4.3 New reference: `docs/reference/eval-report.md` — the report JSON contract with the example.
- [x] 4.4 New explanation: `docs/explanation/why-weak-readers.md` — the `:free`-as-entry-tier rationale and variability-as-signal.
- [x] 4.5 Cross-link every book section to the spec requirement it elaborates (and vice versa).

## 5. Discoverability
- [x] 5.1 Add the new pages to `docs/SUMMARY.md`.
- [x] 5.2 Regenerate `llms.txt` / `llm.txt`; confirm token-estimate stays within the declared budget (degrade if not).
- [x] 5.3 `suite_linter`: add a check that consumer repos' eval docs reference the guideline pages (adoption probe).

## 6. Adoption
- [x] 6.1 Draft per-repo adoption tickets (dont, wai, espectacular, pretender, testaruda): first tier-0 battery + one tier-1 scenario; tier 2 deferred until stable.
- [x] 6.2 Record the guideline decisions in `wai` (why per-consumer harness, why spec-as-source-of-truth).
- [x] 6.3 Record the D8 alignment decisions in `wai`: which AI Evals FAQ practices were adopted (provenance, prevalence bounding, staleness review) and which were deliberately out of scope (LLM-judge validation, production sampling).
