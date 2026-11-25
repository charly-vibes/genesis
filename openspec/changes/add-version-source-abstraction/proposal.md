# Proposal: add-version-source-abstraction

Tracked as `genesis-cn6` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-006`, `DRAFT-002`, `DRAFT-003`, `CLAR-001`, `CLAR-003`, `EDGE-004`,
`EXCL-001`–`EXCL-004`.

Evidence: `genesis/src/update_check.rs:105` (crates.io, feature-gated) and
`dulce-de-leche/src/installer/upgrade.rs:326,365` (npm CLI + GitHub releases
API). The no-network constraint comes from wild's tool contract
(`wild/src/main.rs` — "never … perform network access").

Verification: self-reported validation.

Scope: the fleet's version-checking paths — crates.io (wai, specodelic) and ddL's npm/GitHub-release paths.

Depends on: nothing. Blocked by: nothing. Unblocks: nothing.

Capability path: `update-check` (existing); module: `genesis::update_check`.

Version bump: **patch** — the crates.io path's behavior is unchanged; the new
sources are additive behind the same opt-in feature.

Downstream repos: 1 (dulce-de-leche) — one ticket.

## Why

Version checking is split across the fleet in two unrelated shapes:

- `genesis::update_check` — crates.io, for a tool's own crate, feature-gated
  behind `update-check` (`src/update_check.rs:105`), with a cached-passive,
  fail-silent notice;
- `dulce-de-leche/src/installer/upgrade.rs:326` — `check_latest_version`, which
  resolves a tool's latest version from the **npm registry** (via the npm CLI)
  or the **GitHub releases API**, per tool kind, and
  `check_all_latest_versions` for the whole managed set.

Both answer the same question ("what is the newest published version of X?")
with two implementations, two cache stories, and two failure conventions.

The ddL path also shows why the abstraction must not become the default: `wild`
carries an explicit contract that it never performs network access, and
`update-check` must stay opt-in for that reason.

## What Changes

- New in `genesis::update_check`: a `VersionSource` abstraction over the
  publishers the fleet actually uses — `CratesIo`, `GitHubRelease`, `Npm` —
  with one `latest_version(source, id)` entry point and one semver
  comparison, so "is the installed version behind" is answered identically
  everywhere.
- The existing crates.io path is expressed as `VersionSource::CratesIo`; its
  behavior (cache TTL, fail-silent, notice text) is unchanged.
- The `update-check` feature stays **opt-in**. No tool gains a network call
  by default; tools with no-network contracts (wild) must not enable it.

> **Fleet constraint, not a spec scenario.** The no-network rule is a property
> of *other repos'* build (wild's contract forbids network access), so it
> cannot be asserted inside genesis's own capability contract. It is recorded
> here and in the design; the spec asserts only what genesis can assert — that
> a dependent which does not enable the feature performs no lookup.
- ddL migrates `check_latest_version` onto the shared sources, keeping its
  npm-CLI routing and GitHub-release API calls.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `update-check`: adds the publisher abstraction (`CratesIo`, `GitHubRelease`,
  `Npm`) behind the existing opt-in feature, with one comparison path; the
  crates.io behavior, cache, and notice semantics stay as they are.

## Impact

- **Affected code**: `src/update_check.rs` (sources + comparison), plus the
  feature-gated HTTP path; `Cargo.toml` unchanged (`ureq`/`semver` stay
  optional, feature-gated).
- **Affected specs**: `openspec/specs/update-check/spec.md` (delta in this
  change).
- **Downstream**: `dulce-de-leche` migrates
  `src/installer/upgrade.rs:326,365`; `wai` and `specodelic` (the two tools
  that enable the feature today) keep their behavior.
- **No network access is added to any tool that does not opt in.**
