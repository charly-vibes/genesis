# Tasks: add-evals-guidelines

## 1. Spec capability ratified
- [ ] 1.1 Review and approve the `evals-guidelines` delta (this change) — the rules below implement it verbatim.

## 2. Shared vocabulary (code)
- [ ] 2.1 Add `ErrorTaxonomy::ActionFormatViolation` with code `ERR_ACTION_FORMAT_VIOLATION` and one-line agent-actionable description.
- [ ] 2.2 Round-trip tests for the new code; guard existing codes unchanged.
- [ ] 2.3 `with_model` doc update: verbatim ids including `:free` suffix (ties to the MODIFIED evals requirement).

## 3. Report contract
- [ ] 3.1 Define `report_version` 1 JSON shape (required/optional fields per the interoperable-report requirement).
- [ ] 3.2 Publish two example report fixtures (JSON) checked into the repo for consumers to diff against: one tier-2 row (model id + repetition present), one non-tier-2 row (model id omitted).
- [ ] 3.3 Validate the example round-trips through `ScenarioReport::serialize` (or document the delta if live-only fields apply).

## 4. mdBook — explanation and how-to
- [ ] 4.1 Extend `docs/how-to/evals.md` with the live cadence: tier ladder table (claims per tier), action protocol, bounds.
- [ ] 4.2 New how-to: `docs/how-to/evals-ci.md` — running the ladder in CI: static on push, nightly replay, rotated `:free` matrix; 429 semantics; budget.
- [ ] 4.3 New reference: `docs/reference/eval-report.md` — the report JSON contract with the example.
- [ ] 4.4 New explanation: `docs/explanation/why-weak-readers.md` — the `:free`-as-entry-tier rationale and variability-as-signal.
- [ ] 4.5 Cross-link every book section to the spec requirement it elaborates (and vice versa).

## 5. Discoverability
- [ ] 5.1 Add the new pages to `docs/SUMMARY.md`.
- [ ] 5.2 Regenerate `llms.txt` / `llm.txt`; confirm token-estimate stays within the declared budget (degrade if not).
- [ ] 5.3 `suite_linter`: add a check that consumer repos' eval docs reference the guideline pages (adoption probe).

## 6. Adoption
- [ ] 6.1 Draft per-repo adoption tickets (dont, wai, espectacular, pretender, testaruda): first tier-0 battery + one tier-1 scenario; tier 2 deferred until stable.
- [ ] 6.2 Record the guideline decisions in `wai` (why per-consumer harness, why spec-as-source-of-truth).
