# Proposal: populate-tool-registry

Tracked as `genesis-dyn` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-003`, `EDGE-006`, `DRAFT-002`, `DRAFT-003`, `CLAR-001`, `CLAR-003`,
`EXCL-001`–`EXCL-004`.

Evidence: `genesis/src/discovery.rs:126-138` — today an empty detector path
and an unknown detector type both yield "not detected" (the unknown kind is
logged); the manifest in this repo declares 2 tools (`testaruda`, `wai`) out
of the fleet's 18 Rust CLIs.

Verification: self-reported validation.

Scope: the fleet registry — all 18 Rust CLIs (`fotos` excluded); `testaruda` marked `dep-only`.

Depends on: nothing. Blocked by: nothing. Unblocks: nothing.

Capability path: `discovery` (new); module: `genesis::discovery`.

Version bump: **minor** — a schema change and a new detector kind.

Downstream repos: 18 (one per CLI declaring its own entry) — one ticket each.

## Why

`genesis::discovery` reads a `.genesis/tools.toml` manifest, and the manifest
in this repo declares **two** tools (`testaruda`, `wai`) out of the fleet's
eighteen Rust CLIs. The module is effectively dormant: a tool that wants to
know which suite tools exist, or a dashboard that wants to show the fleet,
reads an almost-empty registry.

Registration is also a known open item (beads `genesis-lu6`: "Register
.espectacular detector in genesis tools").

Filling the registry needs one design decision first: a registry entry means
"this tool exists" — but `discovery` is used to answer "is this tool
*installed* here?". Conflating the two makes a dashboard report health for
tools that are absent.

## What Changes

- The registry schema distinguishes **declared** (the tool exists in the
  suite) from **installed** (the tool is present in this working copy), and
  discovery reports both.
- New detector kind: `command` — the tool's executable is on `PATH` — in
  addition to the existing `file` and `directory` detectors. A `command`
  detector answers "installed" for a tool that ships no marker file.
- `.genesis/tools.toml` in this repo is populated with the fleet: every Rust
  CLI in the suite, with a description and a detector.
- Detection never fails the caller: an absent tool is "not installed", not an
  error.
- **A third outcome is introduced.** Today an empty detector path and an
  unknown detector type both collapse to "not detected"
  (`src/discovery.rs:128-138`). An entry that *cannot* be detected — no
  detector, or a detector kind that does not exist — becomes a distinct
  **invalid** state, which is reported rather than silently swallowed. This is
  a behavior change and is called out here so an implementer knows the state
  machine gains a third value.
- Registration is per-tool: a tool's own repo declares its own entry (the
  `.genesis/` manifest lives with the tool), and this repo's manifest is the
  fleet's reference copy.

## Capabilities

### New Capabilities

- `discovery`: the registry schema and the detection contract — declared
  versus installed, the detector kinds, and the rule that absence is a
  finding, not a failure.

### Modified Capabilities

(none)

## Impact

- **Affected code**: `src/discovery.rs` (+ `src/discovery/` submodule for the
  detector kinds), `src/lib.rs` unchanged.
- **Affected specs**: new `openspec/specs/discovery/spec.md`.
- **Affected data**: `.genesis/tools.toml` in this repo — populated.
- **Downstream**: one ticket per tool repo to declare its own entry; the
  fleet reference copy lives here. `genesis-lu6` is closed by the
  `.espectacular` entry.
- **Open input**: `testaruda`'s entry needs a re-scan of that repo (its
  source tree was unreadable during the audit) before its detector is chosen.
