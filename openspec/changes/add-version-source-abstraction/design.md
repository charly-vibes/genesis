# Design: add-version-source-abstraction

## Context

See proposal.md — Why. State that shapes the approach:

- `src/update_check.rs:105` — `check(crate_name, current_version)` queries the
  crates.io API for the caller's own crate, caches the answer (7-day TTL),
  fails silent, and renders a notice through `notice(&UpdateInfo)`.
- `dulce-de-leche/src/installer/upgrade.rs:326` — `check_latest_version(tool)`
  picks a transport per tool kind: npm packages resolve through the npm CLI
  (`npm view <pkg> version`), everything else through the GitHub releases API
  (`/repos/<repo>/releases/latest`, `tag_name` with the `v` prefix trimmed).
- `wild`'s tool contract forbids network access, so `update-check` must remain
  opt-in for tools like it.

## Goals / Non-Goals

**Goals:**

- One "latest published version" question with three answerers.
- One comparison path, so "behind" means the same thing for every publisher.
- No new network access for any tool that does not opt in.

**Non-Goals:**

- Changing the notice text or the cache policy of the crates.io path.
- Shipping a "check every tool" command in genesis — that stays ddL's job
  (it knows the managed set).
- Making the module a default feature.

## Decisions

**An enum of publishers, not a trait registry.** The alternative is a
`VersionSource` trait with user implementations. Rejected: the fleet needs
three publishers, and a public trait invites third-party sources that would
then have to promise the same fail-silent semantics. An enum keeps the
guarantees provable.

**npm resolves through the npm CLI, as ddL does.** ddL already routes npm
through the npm CLI (`npm view <pkg> version`) and handles Windows via
`cmd /C` (the BatBadBut mitigation). The shared source reuses that behavior
rather than introducing an npm HTTP API client — the CLI is the transport ddL
already trusts.

**GitHub releases strip a leading `v`.** ddL trims it
(`tag.trim_start_matches('v')`). Preserved, so the migrated output is
byte-identical.

**One comparison path via `semver`.** `semver` is already an optional
dependency of the `update-check` feature. The comparison treats a pre-release
as current, matching the crates.io path's existing behavior.

**Feature stays opt-in.** Rationale: `wild` forbids network access by
contract; a default-on feature would break that contract in a shared crate
that wild may one day depend on. The spec states the rule so a future
"default it" proposal has to answer it.

**ddL keeps its managed-set knowledge.** genesis answers "what is the latest
version of X"; ddL answers "which tools do I manage" and aggregates. That
split keeps the domain (the managed catalog) out of genesis, per the
boundary rule.

## Risks / Trade-offs

- [ddL's `check_all_latest_versions` behavior changes subtly] → Mitigation:
  the migration keeps ddL's transport choices and trims; its output must
  match the pre-migration output for the same tool set (asserted in ddL's
  migration ticket).
- [A publisher's API changes shape] → Mitigation: every source fails silent
  to "no version", matching the existing fail-silent rule.
- [The abstraction grows a fourth source] → Mitigation: adding an enum
  variant is a deliberate, reviewable change; the spec's scenario list
  documents the supported set.

## Migration Plan

1. Land the abstraction behind the existing feature (additive, patch bump);
   the crates.io path is re-expressed as `VersionSource::CratesIo` with
   identical behavior.
2. ddL migrates `installer/upgrade.rs` (separate ticket in the ddL repo),
   asserting identical output for its managed set.
3. Rollback: ddL reverts to its local implementation; genesis keeps the
   unused variants.

## Open Questions

- Whether the cache TTL should be per-source or per-crate once more than one
  source is in play — deferred; it changes no externally visible behavior in
  this change beyond the crates.io path, which keeps its current TTL.
