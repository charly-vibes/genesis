# Change: Add suite-wide evals guidelines for consumer tools

## Why

genesis already ships the deterministic eval core — `Scenario`, distractor
checks, `ErrorTaxonomy`, model attribution — and `docs/how-to/evals.md`
teaches the authoring discipline ("never score free-form agent text",
protocol adherence vs task success). But the *operational* rules are
missing, and each consumer repo is one consistency audit away from
hand-rolling its own divergent answers:

- Which cadence runs what: static checks every push, scripted replay
  nightly, live model runs on rotation?
- When live models do run: what action protocol, what bounds, what happens
  on 429, what gets recorded?
- The docs name OpenRouter `:free` models as the entry tier (weakest
  readers — if a weak model parses the envelope and honors the hint, the
  channel is robust), but no consumer repo can adopt that today without
  inventing its own pacing, attribution, and retry semantics — which would
  make cross-tool results incomparable.
- Nothing defines the report contract, so no suite-wide dashboard can
  aggregate per-tool eval outcomes.

The gap is **guidelines, not machinery**: genesis owns the conventions and
the shared vocabulary; each tool repo owns its eval battery and its harness,
implementing the conventions. This keeps domain logic out of genesis
(boundary rule) while preventing convention drift.

## What Changes

### Normative rules (new `evals-guidelines` capability)

A spec capability capturing the suite-wide rules every consumer eval
battery SHALL follow:

1. **Tier ladder** — tier 0 static (no model, every push), tier 1 scripted
   replay (no model, nightly), tier 2 live model runs (scheduled, rotated).
   Each tier states its cadence and what it may claim.
2. **Process-boundary scoring** — checks assert only on signals that
   crossed the process boundary (exit codes, envelopes, filesystem state);
   free-form agent text is never scored.
3. **Scenario authoring standard** — fixture provisioning, contrived
   failure injection, distractors, one fault attribution per check, and
   an empty-trajectory guard per tier-2 scenario (a passive model cannot
   pass by doing nothing).
4. **Live action protocol** — structured per-turn action (`command` +
   `done`), standardized minimum model context (never check logic), one
   re-ask on malformed output (consuming a turn), then
   `ERR_ACTION_FORMAT_VIOLATION` agent fault; turn cap, wall-clock
   timeout, token-estimate budget (`aix` heuristic).
5. **Free-tier entry and attribution** — `:free` model ids as the entry
   tier of an ordered registry; raw model ids recorded verbatim; every
   repetition a distinct row; 429 → one spaced retry → `rate_limited`
   (never a tool fault, never a silent model substitution).
6. **Sandbox confinement** — isolated `HOME`, scrubbed environment,
   network denied by default, tool binary pre-installed, per-command
   timeout.
7. **Interoperable report contract** — a versioned JSON shape for scenario
   reports so suite-wide aggregation works without forking per tool;
   model id and repetition index required for tier-2 rows, optional
   elsewhere (status, fault, and `ERR_*` code are three distinct,
   defined vocabularies).
8. **Fault routing** — tool faults become tickets in the tool's repo;
   agent faults routed on check code × output channel across ≥3 model
   ids (or all configured ids when fewer) point at the output channel
   (AIX work in genesis), not at the model.

### Shared vocabulary (code, small)

- One new `ErrorTaxonomy` variant: `ActionFormatViolation`
  (`ERR_ACTION_FORMAT_VIOLATION`) — shared because fault attribution must
  stay comparable across tools. Distinct from
  `ERR_TOOL_EXECUTION_HALLUCINATION` (a never-executed claim, impossible
  in the live tier where the harness executes every action).

### Documentation (mdBook)

- Extend `docs/how-to/evals.md` with the live cadence section.
- New reference page: the report JSON contract (with an example).
- New how-to page: running the tier ladder in CI — static checks on push,
  nightly replay, rotated free-model matrix (GHA-oriented, budget-aware).

### Explicitly out of scope

- **The live-runner implementation stays out of genesis.** Consumer repos
  implement their own harness per the guidelines (the action protocol and
  report contract keep them interoperable). If ≥2 tools converge on the
  same harness code, extracting it then satisfies the boundary rule — not
  before.
- Per-tool scenario authoring (follow-up tickets per consumer repo).
- AIX-ablation provisioning (existing named follow-up in `evals.rs`).

## Impact

- **New capability: `evals-guidelines`** — the normative rules below.
- **Capability: `evals`** — gains one taxonomy variant; all existing
  requirements unchanged.
- Consumer adoption happens via per-repo tickets referencing this change;
  genesis validates the guidelines only by making the vocabulary and the
  report contract concrete.
