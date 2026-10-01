# Design: add-artifact-provenance

## Context

The `add-aix-eval-loop` change (archived 2026-09-17) shipped receipts, token
budgets, distractor scenarios, and feedback→scenario conversion. This change
completes the artifact-staleness loop it left open: artifacts carry no
provenance, so staleness is undetectable; drift cannot be linted; the eval
loop has no receipt check; and the feedback vocabulary has no kind for
AIX-artifact failures.

## Goals / Non-Goals

- Goals: provenance on every genesis-generated artifact; machine-checkable
  staleness; check for receipt recording; aix-gap feedback vocabulary.
- Non-Goals: revocation semantics (undefined in corpus beyond B082's pairing —
  needs its own design); schema-versioned managed blocks (footer version is
  informational only); changing `parse_envelope` leniency.

## Decisions

### D1 — Footer carries a content hash, not a timestamp (determinism)

The budget-degradation ladder (aix D2) and all generator tests rely on
deterministic output. A timestamp in the default footer would break
`generate_llms_txt` determinism and every snapshot test. Decision: default
footer = `generator_version + content_hash + source_name`; timestamped
variants are separate functions (`*_timestamped`) for callers who want them
(audit trails per B343/B163 where "provenance preserved = auditable" needs
custody time). Trade-off: two variants vs. non-deterministic defaults —
determinism wins; the corpus's own framing (B286: trust = repeatability)
supports it.

### D2 — Footer lives inside markers, hash covers content only

The hash covers the block content between markers, excluding the footer line
itself, so the check is: recompute expected content → hash → compare to
footer. Markers stay stable (`BEGIN:<name>` unchanged) so `has_block` and
existing parsers in consumer repos keep working. Formats: HTML comments for
markdown-ish files (AGENTS.md), the same comment syntax for generated txt
footers (`# provenance: ...`) — a comment in every consumed format.

### D3 — Drift check compares hashes, not full text

`ManagedBlockDrift` regenerates expected content per registered block and
compares the footer hash (fast path) or full text (when no footer exists —
older blocks). It reuses `BlockRegistry` + `LintCheck::run(&repo_root)`.
Finding severity: Warning (drift is not breakage); fix command is
caller-provided per block since only the owning tool knows its sync command.

### D4 — `receipt_records_terminal_outcome` is a plain check function

Same shape as `ok_envelope` / `doc_drift_blindness` (`evals.rs:547-619`):
`impl Fn(&ScenarioResult) -> CheckOutcome`. Fault attribution: missing
receipt on a mutating step is `tool_fault` (the tool owns receipt emission);
a receipt with `terminal_outcome: success` over a nonzero exit is also
`tool_fault` (the lie the envelope exists to catch). No new envelope fields —
`ReceiptMeta` already carries everything.

### D5 — `aix-gap` rides the existing kind machinery

Adding `"aix-gap"` to `VALID_KINDS` (`feedback.rs:72`) gives it typo
suggestions, redaction, and ContextBundle capture for free. The kind is
additive vocabulary; existing scripts are unaffected. Naming follows the
aix report's `feedback --kind aix-gap` recommendation verbatim so the corpus
citations stay resolvable.

## Risks / Trade-offs

- **Footer noise in hand-edited files** → HTML comments are inert in rendered
  markdown; the drift lint flags hand-edits as findings rather than failing.
- **Two new specs (managed-block, suite-linter) created for this change** →
  they document existing+new behavior; purpose lines stay one sentence,
  per the feedback spec's "TBD — update after archive" pattern.
- **Hash algorithm choice** → any stable digest works; use the same hashing
  style as `repro_hash` in feedback context to avoid a second hashing story.

## Migration Plan

No migration: footers are opt-in, receipts are already opt-in, the new kind
and check are additive. Consumer repos adopt footers + drift lint in their own
change (per-repo tickets), exactly like the `update_check` wiring pattern.
