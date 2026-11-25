# Design: add-fleet-adoption-plan

## Context

See proposal.md — Why. State that shapes the approach:

- The fleet is 18 Rust CLIs in `para/areas/dev/gh/charly`; the adoption
  matrix (tool → genesis version → modules used → verification state) was
  produced by a scan of each repo's `genesis::` references and `Cargo.toml`
  pins.
- Four tools depend on no genesis at all (kaya, wild, sunshine,
  funkalization); two are pinned to old APIs (bajan 0.8, evallerina's
  `v0.12.3` tag); five adopt genesis partially (whisper, podras, sad,
  gently, evallerina).
- Beads already tracks three of the rows in *this* repo's tracker
  (`genesis-orq` wai hooks, `genesis-q4k` pretender hooks, `genesis-dvf`
  testeruda feedback) — but the other nine rows are tracked nowhere.

## Goals / Non-Goals

**Goals:**

- Every fleet-adoption finding has an owner repo, a one-line scope, a stated
  blast radius, and a place to track it.
- The index is honest about what was measured and what was not.

**Non-Goals:**

- Implementing any of the rows here. Each is a change in another repository.
- Defining new genesis capabilities. Six other proposals do that.
- Re-measuring the fleet — the scan's numbers are inputs, and the
  `testaruda` gap is a row, not a blocker.

## Decisions

**One change holds the index; the tickets live in the owning repos.** The
alternative is filing all twelve tickets in genesis's tracker. Rejected: a
ticket for "kaya adopts genesis" tracked in genesis is a ticket nobody in
kaya will see. genesis keeps the *index*; the owning repo keeps the work.

**Rows carry a blast radius.** "Adopt `genesis::envelope`" is not actionable
without knowing whether the output shape changes. The row says which. The
sunshine row is the honest example: its envelope kinds (`render`, `audit`) have
no genesis equivalent, so its adoption *changes* the values a consumer sees.

**`testaruda` gets a re-measure row, not a guessed row.** Its source tree was
unreadable during the scan (`ls` on a path `find` could see returned "No such
file or directory"), so its adoption level is genuinely unknown. Guessing a
detector or a module list would put a wrong number into the index.

**Breaking pins are called out as such.** bajan's bump is five minor versions
across a changed `Envelope::success` signature — the row says so, so nobody
reads it as a one-line pin edit.

**Nothing is implemented here.** This change is the tracker-of-record for the
fleet lane; a row is "done" when its ticket is closed in the owning repo.

## Risks / Trade-offs

- [The index drifts from the tools' actual state] → Mitigation: the row names
  its measurement (the scan date and the command that produced it); the docs
  page carries the snapshot date too.
- [A row is never picked up] → Mitigation: each row names an owning repo and
  a tracker, so "not started" is visible rather than invisible.
- [A row turns out to be wrong] → Mitigation: rows are claims with a
  measurement, and the `testaruda` row demonstrates the honest way to record
  uncertainty.

## Migration Plan

1. File one ticket per row in the owning repo's tracker (at approval).
2. The docs page (`document-fleet-adoption-review`) links this index.
3. Rows close as their tickets close; the index is the adoption report of
   record.

## Open Questions

- Whether the four non-adopters should adopt genesis *at all*, or stay
  independent — a product decision, not a technical one. The rows assume
  adoption, since the shared contract is cheaper than four envelope
  dialects; the human may decide otherwise.
