# Change: Add AIX eval loop — receipts, artifact budgets, distractor scenarios, feedback→scenario conversion

## Why

Research mining the AIEWF conference corpus (344 talks; see
`.wai/projects/genesis-foundation/research/2026-09-15-aix-practices-from-the-aiewf-conference-corpus.md`,
full report in `microdancing/conference-analysis/aix-for-genesis-report.md`)
converged on four industry practices that genesis can adopt because it already
owns the substrate:

1. **Receipt contract** (B173, OpenAI): "model proposes, harness commits,
   receipt proves." Most agent failures are silent-success failures — coherent
   output over a broken state. Our envelope records `ok`/`data`/`warnings`/
   `hints` but not *terminal outcome classification* or *evidence* — "silence
   cannot be neutral" and "internal success is not external proof."
2. **Budget-aware context artifacts** (B128, Codex harness): Codex caps the
   available-skills list at 2% of the context window and gracefully degrades
   descriptions. `genesis::aix` generates `llms.txt`/`llm.txt` with no token-cost
   notion; an artifact that overstays its budget silently degrades the agents it
   serves (the ddl evaluation flagged managed-block proliferation in practice).
3. **Model-specific failure modes** (B019, Arize): the instruction-following
   ceiling moved 10× (200 → 2,000–5,000 rules) and failures are now
   model-specific — quiet forgetting, loud refusal, overthink-into-silence, and
   **polite half-finish** (does the work, quits mid-way, sounds confident). Our
   evals module has checks for hint-blindness but none for the other shapes; a
   parseable envelope is the antidote to silent partial success and evals must
   replay per model.
4. **Closed loop** (B268, B192, B190, B085): evals act as merge gates, rerun on
   every model change, and production failures are mined back into the scenario
   corpus. genesis owns both ends — the `feedback` module (agent-reported
   aix-gap/bug bundles) and the `evals` module (Scenario replay) — but nothing
   connects them. This is the same "pipeline exists in docs only" gap the
   ddl-family evaluation identified, occurring *inside* genesis itself.

## What Changes

### `genesis::envelope` — receipt metadata (additive, non-breaking)

- Optional `ReceiptMeta` on `Envelope`: `terminal_outcome`
  (Success/Failure/Timeout/Cancelled), `attempt` + `idempotency_key` (mutating
  commands), `evidence` (what the user-visible edge can verify).
- All new fields `skip_serializing_if = Option::is_none`; existing
  `parse_envelope` and downstream parsers unaffected.

### `genesis::aix` — token-cost reporting and budget degradation

- `TokenCost` estimate on generated artifacts (chars/4 heuristic — see design).
- `--budget` style API: `with_budget(max_tokens)` trims section verbosity
  (first-sentence descriptions, drop optional sections) instead of overflowing.

### `genesis::evals` — distractor injection and doc-drift blindness

- `Scenario::distractor_file()` — fixture files present in the environment but
  not part of the happy path (stale docs, contradictory hints).
- `doc_drift_blindness()` check — agent must trust the tool's envelope over a
  stale managed block / llms.txt section.
- Scenario report records which model replayed it (`--models` matrix support).

### `genesis::feedback` — scenario conversion

- `Scenario::from_feedback_context(bundle, fixtures)` — convert a captured
  failure (ContextBundle + optional caller-supplied fixture files) into a
  replayable Scenario, so every aix-gap report can close with a regression
  scenario. Bundle-only conversion yields a prompt-only scenario — the current
  ContextBundle carries command/exit_code/suggestion footer, not file
  snapshots (see design D4).

## Non-goals

- CI merge-gate wiring in consumer repos (repo-level work; genesis ships the
  library primitives only). Tracked separately — precondition: the whisper
  repo (whisper-bez) currently has no CI at all.
- Per-tool falsifiable value propositions (genesis-ntg L1 slice — the
  "falsifiable value proposition" layer of the Agent Value Alignment epic;
  per-tool repos).
- Managed-block/llms.txt size re-audit of downstream repos (docs task; ride
  EXCL-002 release).
- Any breaking change to existing public APIs — envelope gains optional fields;
  existing `generate_llms_txt`/`generate_llm_txt` signatures are untouched
  (see design D2).

## Release plan

One coordinated 0.x minor release containing all four features. All new API
surface is additive; no existing signature, type shape, or serialized output
changes (golden-file test pins envelope output). EXCL-002 (the pending
Unreleased-changelog-entry decision) gates the release, not this change.
