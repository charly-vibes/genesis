# Proposal: add-envelope-conformance-kit

## Why

Every tool that adopts the shared envelope writes its own compatibility test
(`pretender/tests/genesis_compile_test.rs`,
`vampiro/crates/vampiro-cli/tests/genesis_compatibility.rs`), and each asserts a
different subset of the contract. There is no shared statement of what
"conforms" means, so a tool can claim conformance while emitting an envelope
that a fleet reader cannot parse — which is exactly the class of defect the
hint-channel change (`fix-envelope-hint-channel`) had to fix by hand.

The kit also closes the gap the tolerant reader opens: a tolerant *reader*
cannot tell a tool that emits the wrong *writer* shape apart from a tool that
emits a legacy shape.

## What Changes

- New `genesis::conformance` module — a reusable harness a tool calls from its
  own test suite, asserting the fleet-wide output contract:
  - the envelope shape (`ok`, `envelope_version`, `cli_version`,
    `envelope_kind`, `data`);
  - the error channel (an error envelope carries a non-empty
    `data.remediation` with at least one command);
  - the hint channel (the tool's error envelope resolves to a non-empty hint
    command list through `genesis::evals::parse_envelope`);
  - the exit-code mapping (an `ok: true` envelope exits 0; an `ok: false`
    envelope exits non-zero).
- The kit is a **library**, not a CLI: a tool writes one test that runs its
  own binary and asserts the four properties. No new runtime dependency.
- A genesis-side self-test runs the kit against genesis's own conformance
  fixtures, so the kit cannot rot.

## Capabilities

### New Capabilities

- `conformance`: the fleet-wide output contract a tool can assert in one call
  — envelope shape, error channel, hint channel, exit-code mapping.

### Modified Capabilities

(none)

## Impact

- **Affected code**: `src/conformance.rs` (new), `src/lib.rs` (module decl).
- **Affected specs**: new `openspec/specs/conformance/spec.md`.
- **Downstream**: tools may adopt the kit as a single test (one ticket per
  repo, optional — adoption is not required for correctness).
- **No tool's runtime behavior changes** — this is a test-time capability.
