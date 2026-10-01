# Change: Add artifact provenance and staleness detection for generated AIX artifacts

## Why

genesis generates `llms.txt`, `llm.txt`, and managed blocks that are injected
into agent context across 6+ suite repos, but nothing records *which* generator
version, source, or content produced what is on disk. Agents (and humans) cannot
answer "is this artifact current?" — and the mining of the tv conference corpus
(~3.5M words, see sources below) shows why that matters:

- **Provenance is the strongest cross-corpus theme** (9 talks): Kepler (B167)
  — "the model cannot write the number, it writes a *reference* to the number"
  (atomic provenance); Garry Tan (B261) — "provenance on every fact,
  contradiction checks when new information collides with the old"; B343 —
  "the chain of provenance is broken… provenance preserved = auditable."
- **Staleness is a named top-3 context problem** (B115, B235): "ambiguity,
  staleness, and preference." Genesis injects shared artifacts into every
  context window (B197: "loaded into every context window at the start"), so
  staleness is a recurring cost genesis is positioned to own.
- **Doctor/audit framing** (B083): "agents.md hygiene" is a named discipline;
  today there is no hygiene check for the blocks genesis itself manages.

The closed loop already exists at both ends — `aix` generates, `suite_linter`
lints, `feedback` converts failures to scenarios — but nothing connects them:
no artifact records its provenance, so drift is undetectable, and there is no
`aix-gap` feedback kind to report it with.

Sources: `../tv/transcripts_aieng/txt/` talks B061, B082, B115, B163, B167,
B197, B235, B261, B281, B343 (quoted in `.org` research note, session
2026-10-01); quotes are auto-captions — approximate wording.

## What Changes

### 1. `genesis::managed_block` — provenance footer on injected blocks

```text
<!-- BEGIN:cv-genesis (v0.10.3, sha:1b2c3d4e, source:registry-name) -->
...block content...
<!-- END:cv-genesis -->
```

- Footer fields are **content hash (8 hex of a stable digest), generator
  version, and registry source name** — no timestamp, keeping generation
  deterministic (see design.md).
- Opt-in via builder: default injectors keep byte-identical output until a
  dependent adopts footers, mirroring the `with_receipt` approach.

### 2. `genesis::aix` — provenance footer on generated artifacts

`generate_llms_txt` / `generate_llm_txt` (and bounded variants) append a
`provenance` comment line: generator version, content hash, generation
timestamp **opt-in** (timestamped variants are separate functions so the
existing deterministic tests and the budget-degradation ladder stay intact).

### 3. `genesis::suite_linter` — `ManagedBlockDrift` check

New `LintCheck`: for every registered block, regenerate expected content and
compare against disk (via footer hash where present, full-text otherwise).
Findings carry a fix command (`→ Run: <tool> <block-sync-command>`), reusing
the existing `LintResult::with_fix` plumbing and the `LintCheck` registry —
no new trait surface.

### 4. `genesis::evals` — `receipt_records_terminal_outcome` check

A composable `ScenarioCheck` asserting that a run's final envelope carries a
`receipt.terminal_outcome`. This closes the loop on the already-shipped
`ReceiptMeta` (envelope spec, add-aix-eval-loop D1): receipts exist but no
check gates on them.

### 5. `genesis::feedback` — `aix-gap` kind

`VALID_KINDS` gains `"aix-gap"`, routed through the existing feedback flow
(kind validation with typo suggestions, redaction, ContextBundle capture). A
drift or stale-artifact failure reported as `--kind aix-gap` flows through the
existing `Scenario::from_feedback_context` conversion — completing the
failures→scenarios loop for AIX artifacts specifically.

## Impact

- **Affected specs**: `managed-block` (new), `suite-linter` (new), `aix`
  (modified — footer requirement), `evals` (modified — new check), `feedback`
  (modified — kind vocabulary)
- **Affected code**: `src/managed_block.rs` (footer + opt-in builder), `src/aix.rs`
  (footer line + timestamped variants), `src/suite_linter.rs` (new check),
  `src/evals.rs` (new check fn), `src/feedback.rs` (kinds const)
- **Not in scope**: claims decomposition (separate change), cross-suite
  contract-test utility (separate change), revocation semantics (design
  question, tracked separately)
- **Follow-ups (beads)**: CI eval-gate wiring per consumer repo; falsifiable
  value-props as StatusContributor conditions (genesis-ntg territory)
