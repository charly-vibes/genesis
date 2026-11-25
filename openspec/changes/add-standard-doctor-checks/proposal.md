# Proposal: add-standard-doctor-checks

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
