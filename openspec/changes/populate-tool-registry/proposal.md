# Proposal: populate-tool-registry

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
