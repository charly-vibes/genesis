# Tasks: add-aix-eval-loop

Each task is a red→green→refactor cycle: write the failing test from the spec
scenario first, then implement, then tidy in a separate commit.

## 1. Envelope receipt metadata (design D1)

- [ ] 1.1 Define `TerminalOutcome` enum (Success/Failure/Timeout/Cancelled) + `ReceiptMeta` struct; unit test round-trip serialization.
- [ ] 1.2 Add `receipt: Option<ReceiptMeta>` with `skip_serializing_if`; golden-file test (committed serde snapshot) proving envelopes without receipt serialize byte-identically to pre-change output.
- [ ] 1.3 Add `Envelope::with_receipt()` builder; test `parse_envelope` accepts enriched envelopes with unchanged return type.
- [ ] 1.4 Property test: serde round-trip preserves all receipt fields.
- [ ] 1.5 Update docs/reference/modules.md envelope section + CHANGELOG compat note.

## 2. AIX token cost + budget (design D2)

- [ ] 2.1 Define `TokenCost { estimate, heuristic }`; `estimate_token_cost(&str)` free fn (chars/4); unit tests (monotonicity on added modules).
- [ ] 2.2 Verify existing `generate_llms_txt` / `generate_llm_txt` compile unchanged; golden-file test pinning current output.
- [ ] 2.3 Implement `generate_llms_txt_bounded(meta, modules, budget)` degradation stage 1: truncate module descriptions to first sentence. Test: under-budget input is byte-identical to unbudgeted.
- [ ] 2.4 Implement `generate_llm_txt_bounded(title, description, sections, budget)`; degradation stage 2: drop raw/optional sections before tables; headings always survive. Test each level independently on both generators.
- [ ] 2.5 Determinism test: same inputs + budget → byte-identical outputs (10 iterations).
- [ ] 2.6 Document heuristic ±25% band in rustdoc + modules.md.

## 3. Evals distractors + doc-drift check (design D3)

- [ ] 3.1 Define `DistractorKind` (StaleDocs, ContradictingHint); `Scenario::distractor_file()`; test distractor lands in replay fixture env.
- [ ] 3.2 Test distractor-ignoring run passes (no false fault).
- [ ] 3.3 Implement `doc_drift_blindness()` check: pass on envelope-trusting steps; test the pass path.
- [ ] 3.4 Test the fail path: doc-following steps → `agent_fault` with distractor path in reason.
- [ ] 3.5 Add `model: Option<String>` to `ScenarioReport` + builder; test serialized presence/absence.
- [ ] 3.6 End-to-end scenario: stale managed block vs live `--help` envelope — agent must follow the envelope (this is the research's doc-drift scenario, corpus B171/B128).

## 4. Feedback → Scenario conversion (design D4)

- [ ] 4.1 Implement `Scenario::from_feedback_context(bundle, fixtures)` bundle-only case (command + expected error envelope with footer hint); test replay against recorded steps.
- [ ] 4.2 Implement fixture path: caller-supplied (path, content) pairs become scenario fixtures; test converted scenario replays.
- [ ] 4.3 Implement `from_last_error(tool_name, fixtures)` wrapping `scratch::read_last_error`; typed error when no record exists.
- [ ] 4.4 Test no-LLM/no-subprocess guarantee: conversion + replay run in-process.
- [ ] 4.5 Wire a worked example into feedback docs (aix-gap bundle → regression scenario).

## 5. Tidy + validation

- [ ] 5.1 Refactor pass: extract shared fixture-env logic if 3.x introduced duplication; clippy --all-targets clean.
- [ ] 5.2 Full suite green; `pretender check` clean.
- [ ] 5.3 `openspec validate add-aix-eval-loop --strict` clean; mark tasks complete.

## Dependencies

- 1.x, 2.x, 3.x, 4.x are mutually independent — fully parallelizable.
- 5.x last. No consumer-repo changes in this change (see proposal non-goals).
