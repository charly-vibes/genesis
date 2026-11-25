# Proposal: document-fleet-adoption-review

## Why

A fleet-wide review of the charly Rust CLIs against `genesis-vibes` was run to
decide whether genesis needs more surface area. The verdict is yes — but the
first write-up had four defects that a reader could not see, and three of them
were load-bearing:

- the highest-value finding (the envelope hint-channel defect, which lives
  **inside** genesis) was filed under "fleet drift" and read as a set of tool
  bugs, so the fix would have landed in the wrong repos;
- one claim was false (bajan "hand-rolls shell completions" — those were
  OpenAI chat-completions strings);
- two counts were wrong (atomic-write sites: 12 in 7 tools, not 13 in 8;
  `genesis::doctor` adopters: 9, not 8);
- findings carried no identifiers, so no ticket, wai entry, or CHANGELOG line
  could reference one.

A review whose output is meant to seed tickets needs to be addressable,
corrected, and honest about what was verified.

## What Changes

- New explanation page `docs/src/explanation/fleet-adoption.md`, linked from
  `docs/src/SUMMARY.md`, carrying the review's *second* version:
  - the adoption matrix (tool → genesis version → modules used → verification
    state), with the unverified rows marked as such;
  - every finding with a stable id (`F-01`…`), its location, and the command
    that reproduces it;
  - the three owner-lanes — work that lands in **genesis**, work that lands in
    a **tool repo**, and **fleet contract** changes — with the blast radius
    of each;
  - the corrected counts, with the reasoning that changed them;
  - the verification record: which findings were checked by an independent
    reader, which were below its confidence gate, and which remain unmeasured.
- The page states the adoption *rule* in one place: a capability belongs in
  genesis only if two or more suite tools need it, and a tool that needs
  nothing shared stays out.

## Capabilities

### New Capabilities

(none — documentation only; `skip_specs: true`)

### Modified Capabilities

(none)

## Impact

- **Affected code**: none. Documentation only.
- **Affected docs**: new `docs/src/explanation/fleet-adoption.md`; one line
  added to `docs/src/SUMMARY.md`.
- **Affected specs**: none — this change declares `skip_specs: true` because
  no system behavior changes.
- **Follow-up**: the findings on the page become the bd tickets and the
  OpenSpec changes that implement them; the page is the index of that work.
