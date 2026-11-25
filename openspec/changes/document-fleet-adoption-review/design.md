# Design: document-fleet-adoption-review

## Context

See proposal.md — Why. State that shapes the approach:

- The review ran over 18 Rust CLIs in the charly suite (wai, dont,
  dulce-de-leche, testeruda, espectacular, pretender, specodelic, whisper,
  vampiro, podras, evallerina, bajan, sad, gently, kaya, wild, sunshine,
  funkalization) plus `fotos` (a GUI app, out of scope) and genesis itself.
- The review passed five editorial stages; each stage corrected the previous
  one. The corrections that survived are the ones the page must record.
- The repo's docs live in `docs/src/` (mdBook), with `explanation/` for
  "why" pages (`architecture.md`, `design-decisions.md`,
  `agent-instruction-boundaries.md`, `why-weak-readers.md`).

## Goals / Non-Goals

**Goals:**

- A page a maintainer can read to learn what genesis owns today, what the
  fleet has adopted, what is proposed, and which claims are verified.
- Every finding addressable by id, with a reproduction command.
- The owner-lane split, so a reader knows which repo a fix lands in.

**Non-Goals:**

- Duplicating the OpenSpec change proposals. The page indexes them; each
  proposal carries the detail.
- A live dashboard of adoption. The matrix is a snapshot with a date.
- Restating the module reference (`reference/modules.md` already does that).

## Decisions

**The page indexes the proposals.** The proposals are the source of truth for
scope; the page carries the *map* — finding id, one line of claim, location,
lane, and repro command. Rationale: a page that restates proposals rots
faster than the proposals do.

**Findings get stable ids (`F-01`…).** Rationale: bd issues, wai entries,
CHANGELOG lines, and later reviews reference findings by id. The ids are
assigned once, in the page, and never renumbered — a retired finding is marked
retired, not deleted.

**Verification state is on the page.** Each finding carries its state:
`verified` (independently checked), `review-required` (checked below the
confidence gate), or `unverified` (below the check threshold; self-reported).
Rationale: the review's credibility depends on the reader knowing which
claims were confirmed by something other than the author's own reading.

**The matrix marks unreadable rows.** `testaruda`'s source tree was not
readable during the scan, so its row carries a `dep-only` state rather than an
adoption claim. Rationale: a fleet matrix with one silently wrong row is worse
than one that admits a gap.

**Counts carry their correction.** The page states the corrected numbers *and*
the reasoning that changed them (e.g. "12 sites in 7 tools — `sunshine`'s
`cli.rs` calls `stamp::atomic_write` rather than implementing it"). Rationale:
a number without its derivation is a number a reader has to re-derive.

**The adoption rule is stated once, on the page.** Two or more tools need it,
or it stays in the tool. Rationale: the rule is what keeps genesis from
becoming a junk drawer; it belongs where the matrix is read.

## Risks / Trade-offs

- [The page goes stale as adoption changes] → Mitigation: the page states its
  snapshot date and points at the proposals as the live record; a drift check
  is not attempted (the page is prose, not data).
- [The page duplicates content that belongs in the proposals] → Mitigation:
  the page holds one line per finding plus an index; scope detail stays in
  the proposal.
- [Finding ids drift from the bd issue ids] → Mitigation: each finding names
  the bd issue it becomes; ids are assigned once.

## Migration Plan

1. Write the page and link it from `SUMMARY.md` (docs only).
2. The page's findings become the bd tickets (one per finding) and the
   OpenSpec proposals (already created for the genesis-side work).
3. Rollback: delete the page and the `SUMMARY.md` line.

## Open Questions

- Whether the page should also carry the *rejected* candidates (the things
  deliberately kept out of genesis: wild's v1 envelope, podras' health
  signals, funkalization's tracer) — deferred; it changes no behavior, and
  the "not candidates" list is short enough to add later.
