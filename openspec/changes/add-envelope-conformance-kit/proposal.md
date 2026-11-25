# Proposal: add-envelope-conformance-kit

Tracked as `genesis-ffc` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-005`, `DRAFT-002`, `DRAFT-003`, `CLAR-001`, `CLAR-002`, `CLAR-003`,
`EDGE-005`, `EXCL-001`–`EXCL-004`.

Evidence: `pretender/pretender/tests/genesis_compile_test.rs:25-133` (envelope
shape, suggestions, managed-block footer, registry names) and
`vampiro/crates/vampiro-cli/tests/genesis_compatibility.rs:25-116` (suggestions,
managed block, scaffold, discovery) — the two suites cover **different
subsets**, and neither asserts the error channel or the exit-code mapping.

Verification: self-reported validation (the review's independent pass covered
the fleet-adoption findings, not this kit's design).

Scope: the tools that depend on genesis (13 of 18) and their local compatibility suites.

Depends on: `fix-envelope-hint-channel` — the kit's hint check resolves
through the dual-channel reader that change introduces. Blocked by: nothing
(it can land first, but the hint check is then weaker). Unblocks: nothing.

Capability path: `conformance` (new); module: `genesis::conformance`.

Version bump: **minor** — a new public module.

Downstream repos: 8 (pretender, vampiro, wai, dulce-de-leche, specodelic,
whisper, espectacular, dont) — one ticket each, optional.

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
- **New versus already covered.** The *shape* check is partly covered today
  (pretender's suite asserts `ok` and `envelope_kind`); the **error channel**
  and the **exit-code mapping** are covered by no existing suite — those are
  the genuinely new properties. The kit therefore complements the local
  compatibility tests rather than replacing them wholesale.
- A genesis-side self-test runs the kit against genesis's own conformance
  fixtures, so the kit cannot rot.

### Public API (sketch)

```rust
use genesis::conformance::{assert_conforms, conformance_report};

let report = conformance_report(&captured_stdout, captured_exit_code);
assert_conforms(&report, &["shape", "error", "hint", "exit"])?;
```

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
  repo, optional — adoption is not required for correctness). The existing
  local suites keep their domain assertions (suggestions, managed blocks,
  scaffold) and gain the error-channel and exit-code properties.
- **No tool's runtime behavior changes** — this is a test-time capability.
