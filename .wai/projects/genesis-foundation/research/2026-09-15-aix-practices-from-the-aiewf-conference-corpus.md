# AIX practices from the AIEWF conference corpus → genesis-vibes

**Date:** 2026-09-15
**Question:** what practices from industry (mined from the
`microdancing/conference-analysis` corpus — 344 AIEWF talks, ~1.33M words) can
genesis-vibes leverage for its agent-experience (AIX) surface?
**Full artifact:** `../../../microdancing/conference-analysis/aix-for-genesis-report.md`
(Rule-of-5 reviewed to convergence, verdict READY)

## Why this research

genesis owns the tool-family AIX substrate (envelope, self-healing errors,
managed blocks, llms.txt, evals, doctor/status/feedback). The corpus contains a
dense harness/evals/context cluster (67 AIX-relevant talks, 22% evals + 18%
context/harness of AIEWF). Before scheduling the ntg (Agent Value Alignment)
breakdown and the planned AIX-ablation slice, we needed to know which industry
practices map onto the existing substrate — so tickets reference evidence, not
invention.

## Method

Targeted transcript mining of the harness (B007, B173, B128), evals (B085,
B190, B192, B198, B268), instruction-ceiling (B019), and skills-contract (B166,
B171) clusters; cross-checked against `src/aix.rs`, `src/evals.rs`,
`.wai/projects/genesis-foundation/research/*`, and the ddl-family evaluation.
Load-bearing numbers verified verbatim against transcripts.

## Findings (4 adoption themes)

1. **Receipt contract** (B173 OpenAI): envelope should mature from status object
   to receipt — terminal-outcome class, attempt/idempotency, evidence pointer.
   Additive; extends the genesis-u40 exit-code contract.
2. **Budget-aware AIX artifacts** (B128 Codex): skills capped at 2% of context
   window with graceful degradation → `aix.rs` should report artifact token cost
   and support `--budget` trimming; makes AIX-ablation measurable (delta/token).
3. **Instruction ceiling moved 10×** (B019 Arize): 200→2,000–5,000 rules; failure
   modes now model-specific (quiet forget / refusal / overthink / polite
   half-finish). Parseable envelopes are the antidote to silent partial success;
   evals need a per-model matrix; old size assumptions (essentials distillation)
   are stale.
4. **Closed loop** (B268, B192, B190, B085): evals as merge gates; rerun on every
   model change; production failures (feedback module) convert into replayable
   Scenarios. Nothing currently connects feedback→evals inside genesis.

Plus confirmations: falsifiable VPs as StatusContributor conditions (unifies ntg
L1+L4); corpus-backed positioning "observability + contract layer for agent-first
CLI tools"; distractor scenarios (stale-managed-block → doc-drift blindness).

## Decision/recommendation

Seven prioritized recommendations in the full report (CI eval-gate = cheapest
high-value; envelope receipt = only schema change, rides next minor release).
**Reconciliation:** the ddl-family evaluation's "fund the spine / demote
init-and-forget" verdict governs adoption effort; this research funds the
substrate — the evals-gate *is* the "falsifiable metric or demotion" mechanism
the ddl evaluation demanded. Not in conflict.

## Follow-ups

- File bd tickets for the 7 recommendations (ntg child slices where applicable)
- ntg breakdown should incorporate the falsifiable-VP-as-StatusContributor shape
- EXCL-002 decision gates release of recommendation #3 (envelope receipt fields)

