# Proposal: add-standard-doctor-checks

Tracked as `genesis-ykp` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-004`, `DRAFT-002`, `DRAFT-003`, `CLAR-001`, `CLAR-003`, `EDGE-005`,
`EXCL-001`–`EXCL-004`.

Evidence: 9 tools ship a doctor through `genesis::doctor` — wai, dont,
dulce-de-leche, vampiro, pretender, espectacular, specodelic, whisper,
gently. The same check name means different things across them: `config`
means `.vampiro/config.toml` in vampiro
(`vampiro/crates/vampiro-cli/src/doctor.rs:20-45`) and "store opened" in dont
(`dont/src/doctor_checks.rs:25-60`).

Verification: self-reported validation.

Scope: the 9 tools that ship a doctor through `genesis::doctor`.

Depends on: nothing. Blocked by: nothing. Unblocks: nothing.

Capability path: `doctor` (new); module: `genesis::doctor` (+ `standard`
submodule).

Version bump: **minor** — a new public module and a new result state.

Downstream repos: 9 (the doctor adopters) — one ticket each, optional.

## Why

Nine fleet tools ship a `doctor` command through `genesis::doctor`, and each
one re-implements the same four checks with its own naming and its own
meaning: "config" means `.vampiro/config.toml` in vampiro
(`vampiro/crates/vampiro-cli/src/doctor.rs:20-45`) and "store opened" in dont
(`dont/src/doctor_checks.rs:44-60`); "managed blocks" means the tool's own
blocks in each repo. The result is that a reader comparing two tools' `doctor`
output cannot tell whether "config: OK" means "checked and fine" or "not
applicable".

A shared battery is only useful if it cannot lie, which means every check must
be parameterized by the tool that uses it — a check the tool did not configure
must not report "healthy".

## What Changes

- New `genesis::doctor::standard` — a parameterized battery of checks every
  tool can opt into:
  - `config_check(path)` — the configured config file exists and parses;
  - `in_git_repo_check()` — the working directory is inside a Git repository;
  - `managed_blocks_check(block_names)` — the named managed blocks match their
    injected state (drift detection);
  - `ci_workflow_check(path)` — the configured CI workflow file exists.
- Each check reports **not-applicable** when the tool did not configure it,
  instead of "healthy". A report that mixes "ok" and "n/a" is comparable
  across tools; a report that says "ok" for an unconfigured check is not.
- The battery composes with the existing `DoctorCheck` trait and the existing
  `genesis::status` bridge (`StatusBuilder::register_doctor`), so a tool's
  `status` reflects the same checks as its `doctor`.
- **Serialization impact (stated, not hidden):** the result *enum* gains a
  third member, so the serialized `doctor` output gains a state. Existing
  checks keep their behavior, but consumers that match on the state (ddL's
  diagnostics report, evallerina's tool-shape checks) must tolerate an
  unknown state rather than treating it as a failure. The `doctor` spec
  states that tolerance rule.
- No existing check changes behavior; this is additive.

## Capabilities

### New Capabilities

- `doctor`: the diagnostic framework and the shared, parameterized check
  battery — including the rule that an unconfigured check reports
  not-applicable, never healthy.

### Modified Capabilities

(none)

## Impact

- **Affected code**: `src/doctor/standard.rs` (new), `src/doctor.rs`
  (module decl), `src/lib.rs` unchanged otherwise.
- **Affected specs**: new `openspec/specs/doctor/spec.md`.
- **Downstream**: tools may adopt the battery (one ticket per repo, optional —
  a tool's own checks keep working).
- No tool's runtime behavior changes unless it opts in.
