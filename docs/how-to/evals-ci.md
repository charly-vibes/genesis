# Running the Eval Tier Ladder in CI

**TL;DR:** Tier 0 on every push (no model, fast), tier 1 replay nightly (no
model, recorded transcripts), tier 2 live on a schedule with a rotated
`:free` matrix and a hard budget. Live trials never gate a push.

*(Elaborates the **Tier ladder** requirement of the `evals-guidelines` spec —
`openspec/specs/evals-guidelines/spec.md`.)*

## Context & Prerequisites

- An eval battery authored per [Creating Agent Evals](evals.md)
- The tier definitions and their claims: [Step 8: Run the tier ladder](evals.md#step-8-run-the-tier-ladder)
- Reports emitted per [the eval report contract](../reference/eval-report.md)

## Tier 0 — static checks on push

No model is involved, so this tier belongs in the normal test job and gates
the push:

- Envelope shape and schema checks over golden fixtures
- Doc-sync guards (`llms.txt` / `llm.txt` staleness)
- Scenario declarations well-formed: every scenario names its provenance
  (corpus entry, error-analysis note, or ticket), every declared check is
  present in the report

Runtime: seconds. May claim only "the channels are well-formed".

## Tier 1 — nightly scripted replay

`Scenario::run` over recorded `AgentStep` transcripts — deterministic, no
model, no network. Schedule it nightly (GitHub Actions `schedule`), not on
push: it exercises detection of every recorded failure mode and takes
minutes, not seconds.

```yaml
# .github/workflows/evals-replay.yml (sketch)
on:
  schedule:
    - cron: "0 3 * * *"   # nightly, off-peak
jobs:
  replay:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo test --test evals_replay -- --nocapture
      # Rows append to the report store in report_version 1 shape.
```

May claim "recorded failure modes are still detected". A replay failure is a
regression signal — triage it as the fault it reports (tool faults become
tickets; agent faults route by check code × output channel).

## Tier 2 — live, scheduled, rotated

Live model runs are a separate scheduled workflow that never gates a push:

1. **Rotation** — iterate the ordered model registry; `:free` ids run first
   (entry tier). Order defines scheduling priority, not failover: fallback to
   a different model is a caller-level decision, recorded per trial.
2. **Repetitions** — n ≥ 3 per scenario × model cell; each repetition a
   distinct report row (`model` verbatim, `repetition` 0-based).
3. **Action protocol** — one structured action per turn, one re-ask on
   malformed output, then `ERR_ACTION_FORMAT_VIOLATION`.
4. **Bounds** — turn cap, wall-clock timeout, token-estimate budget
   (`aix` chars/4 heuristic); applied values recorded in every row.

### 429 semantics

On HTTP 429 from the model provider: retry **once** after a delay of at
least two seconds (record the delay used as `retry_delay_secs`); a second
429 ends the trial `rate_limited` — no fault attribution, no silent
substitution of another model. Never let a rate-limited cell silently
disappear: the row is the record that the cell did not run.

### Budget

Cap tier-2 spend before the first run:

- A **token-estimate budget per trial** (the `aix` chars/4 heuristic, labeled
  as heuristic) — recorded in `bounds.token_budget`; exceeding it stops the
  trial `failed` with `bound_reached: "token_budget"` and no fault
  attribution.
- A **per-run wall-clock budget** for the workflow itself — if the schedule
  slips past the window, remaining cells are skipped and recorded as absent,
  not failed.
- Free-tier ids keep the marginal cost near zero; paid ids enter the
  registry only when the claim they buy justifies their cost.

## What each tier buys you

| Tier | Catches | Never claims |
|---|---|---|
| 0 | Broken channels, drifted docs | anything about agent behavior |
| 1 | Lost detection of known failure modes | current-model behavior |
| 2 | Current agent behavior, directional | regression gating, prevalence rates |

A permanently green battery — any tier — is reviewed for staleness, not
celebrated: checks that never fail are re-sharpened or retired.

## Related

- [Creating Agent Evals](evals.md) — authoring scenarios and the live action protocol
- [Why weak readers](../explanation/why-weak-readers.md) — why `:free` ids are the entry tier
- [The eval report contract](../reference/eval-report.md) — the JSON rows all tiers emit
