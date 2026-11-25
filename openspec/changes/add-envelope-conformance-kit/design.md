# Design: add-envelope-conformance-kit

## Context

See proposal.md — Why. State that shapes the approach:

- `pretender/tests/genesis_compile_test.rs` and
  `vampiro/crates/vampiro-cli/tests/genesis_compatibility.rs` each assert a
  different subset of the envelope contract; neither is a fleet-wide
  statement.
- `genesis::evals::parse_envelope` is the reader the fleet's evaluations use
  (`src/evals/outcome.rs:38`) — the kit must assert through *that* reader, not
  a private one, or it proves nothing.
- The kit runs at test time, inside a consumer's own crate, so it cannot
  depend on anything the consumer's dev-dependencies lack.

## Goals / Non-Goals

**Goals:**

- One call that asserts the whole output contract and reports **every**
  violated property, not just the first.
- The kit's hint assertion goes through the shared reader, so a tool cannot
  pass with a shape the fleet's evaluations cannot read.

**Non-Goals:**

- A CLI (`conformance check`) — the kit is a library called from a test.
- Evaluating *content* (is this remediation a good suggestion?). The kit
  asserts presence and resolvability, not quality.
- Per-tool fixtures. The kit asserts properties of whatever output the caller
  captured.

## Decisions

**Library, not CLI.** Rationale: the assertions must run inside the
consumer's test process, where the tool's own types and fixtures live. A
subprocess CLI would need a serialization contract for the failures — more
surface, less signal.

**Assert through the shared reader.** The hint check calls
`genesis::evals::parse_envelope` (post `fix-envelope-hint-channel`, the
dual-channel reader). Alternative: a private copy of the parsing logic in the
kit. Rejected — a private copy is exactly how the three hint-channel variants
appeared in the first place.

**Report all violations, not the first.** A tool author fixing conformance
should see the whole list in one run. Rationale: the checks are independent
properties; short-circuiting hides the second bug until the next run.

**Four properties, kept small.** Shape, error channel, hint channel, exit
code. The alternative — a broader "suite" — was rejected because each
additional property multiplies the false-fault rate against tools that
legitimately differ (e.g. a tool with no hints on a success path).

**The kit checks the writer shape, the reader checks tolerance.** A tool that
emits a legacy top-level error-path hint array fails the kit's error-channel
check (remediation missing) even though the shared reader tolerates it. That
split is deliberate: the reader is tolerant for legacy producers, the kit
holds new producers to the contract.

## Risks / Trade-offs

- [A tool with a legitimately different contract is forced to change] →
  Mitigation: the kit is opt-in per tool; adoption is a test-only change.
- [The kit's expectations drift from the envelope spec] → Mitigation: the
  genesis-side self-test runs the kit against genesis's own fixtures, and the
  spec requirements name the same properties.
- [The kit asserts exit-code mapping that a tool with a special exit code
  (e.g. "needs attention" = 2) rejects] → Mitigation: the check compares
  success/non-success, not the exact code.

## Migration Plan

1. Land `genesis::conformance` plus its self-test (additive, patch bump).
2. Tools adopt it as one test each (optional, one ticket per repo).
3. Rollback: removing the module breaks only adopters' test code.

## Open Questions

- Whether the kit should also assert the *documentation* properties (e.g. the
  `AGENTS.md` managed block present) — deferred; that is the `doctor`
  battery's job, not the output contract's.
