# Design: add-standard-doctor-checks

## Context

See proposal.md — Why. State that shapes the approach:

- `genesis::doctor` already provides `DoctorCheck` (a check returning
  `Vec<LintResult>`), `DoctorRunner`, and `DoctorReport` with an auto-fix
  mode; `genesis::status::StatusBuilder::register_doctor` already bridges a
  runner into the dashboard.
- The framework's result type today carries severity (`Error`, `Warning`,
  `Advisory`) but no "not applicable" state — that state has to be added for
  the battery to be honest.
- Tool checks today own their own vocabulary: vampiro's `config` check
  (`vampiro/crates/vampiro-cli/src/doctor.rs:20-45`) versus dont's `substrate`
  check (`dont/src/doctor_checks.rs:44-60`).

## Goals / Non-Goals

**Goals:**

- One parameterized battery that produces comparable results across tools.
- A result vocabulary that can say "not applicable", so no tool can claim
  health for an unconfigured check.
- Zero behavior change for tools that do not adopt.

**Non-Goals:**

- Replacing tool-specific checks. A tool's domain checks (store open, corpus
  present, catalog reachable) stay in the tool.
- A fleet-wide "doctor" CLI. Each tool keeps its own command.
- Auto-fix recipes for the standard checks (the framework already has an
  auto-fix mode; wiring fixes for the battery is deferred).

## Decisions

**Not-applicable is a first-class result.** The alternative is to omit
unconfigured checks from the report. Rejected: an omitted check is invisible,
and a reader cannot distinguish "nothing to check" from "check not run". A
visible `n/a` state is comparable across tools, which is the whole point of
the battery.

**Checks are configured, not hard-coded.** Each check takes the tool's own
inputs (config path, block names, workflow path). A hard-coded list would
produce a battery that is wrong for most tools — the exact failure that
motivates the change.

**The battery composes with `DoctorCheck`.** The battery is a set of
`DoctorCheck` implementations, so a tool mixes standard and domain checks in
one runner and gets one report. A parallel "standard report" type would
double the reporting code.

**Severity vocabulary unchanged.** The new state is an additional result
state, not a new severity: an existing check that reports `Advisory` for
"fine" keeps doing so. Rationale: changing the severity vocabulary would
break every consumer's parsers for no gain.

**Status bridge unchanged.** `StatusBuilder::register_doctor` already exists;
the battery only has to produce results the bridge can carry, including the
new not-applicable state (rendered as an item without a failure).

## Risks / Trade-offs

- [A tool's report gains rows it did not have, changing parsers] →
  Mitigation: adoption is opt-in; the not-applicable state is additive.
- [The not-applicable state is new to consumers of `LintResult`] → Mitigation:
  it is additive to the enum, and the fleet's consumers are in-repo and
  updatable; the deployed `doctor` spec pins the vocabulary.
- [Two tools configure the same check differently, so reports still differ] →
  Mitigation: accepted — the battery standardizes the *checks*, not the
  configuration; the config path is the tool's own fact.

## Migration Plan

1. Land the battery plus the not-applicable state (additive, patch bump).
2. Tools adopt selectively (one ticket per repo, optional).
3. Rollback: adopters revert to their local checks; the module can stay.

## Open Questions

- Whether the battery should include a "no uncommitted changes" or
  "remotes reachable" check — deferred; those are git-domain checks, and
  `genesis::git` (read-only plumbing) is the right home, not the battery.
