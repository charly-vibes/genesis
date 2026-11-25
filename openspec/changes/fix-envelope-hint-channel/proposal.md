# Proposal: fix-envelope-hint-channel

Tracked as `genesis-jsu` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-001`, `CORR-002`, `CORR-007`, `EDGE-001`, `EDGE-002`, `DRAFT-002`,
`DRAFT-003`, `CLAR-001`, `CLAR-003`, `EXCL-001`–`EXCL-004`.

Evidence: `src/envelope.rs:311-319`, `src/evals/outcome.rs:38-66`,
`src/evals/tests_e2e.rs:11-22`, `evallerina/src/envelope.rs:60-84`, and a
scan of the fleet's envelope writers (`wai/src/output.rs:12`,
`dulce-de-leche/src/output.rs:103`, `kaya/src/envelope.rs`,
`sunshine/src/envelope.rs`).

Verification: the central claim was independently checked (confidence 0.87,
above the 0.8 gate). The error-path-hints rationale was **re-derived from the
code after review** — see the note in "What Changes".

Scope: the fleet's envelope writers and readers — 18 Rust CLIs scanned (`fotos`, a GUI app, excluded); scan date 2026-11-25.

Depends on: nothing. Blocked by: nothing. Unblocks `add-envelope-conformance-kit`
(which asserts hint resolvability through this reader).

Capability path: `envelope` and `evals` (existing); module: `genesis::evals`.

Version bump: **patch** — reader behavior changes, no public signature changes.

Downstream repos: 1 (evallerina) — one ticket; plus the fleet's recorded
evaluation corpus, which this change can flip classifications for.

## Why

`genesis::envelope::Envelope::error` always sets `hints: None` and carries the
remediation inside `data` (`data: err`, where `ErrorResult.remediation` holds
the `command`/`description` pairs) — `src/envelope.rs:311-319`. But
`genesis::evals::parse_envelope` reads hint commands **only** from the
top-level `hints` array — `src/evals/outcome.rs:52-60`. So no
genesis-conformant error envelope can ever satisfy the hint-following checks
(`hint_followed`, `error_envelope_with_remediation_hint`). The only reason the
e2e test passes is that it fabricates a shape genesis never emits
(`{"ok":false,"data":{…},"hints":[{…}]}` — `src/evals/tests_e2e.rs:11-18`).

`evallerina` already worked around it with a private dual-channel reader
(`evallerina/src/envelope.rs:60-84`). Until this is fixed, the fleet's
"agent follows the hint" loop is unverified end-to-end, and the next tool that
writes its own reader will write a third variant.

Evidence: Rule-of-5 review of the fleet adoption report, TypeSafe-verified
(choice `verified`, confidence 0.87 ≥ the 0.8 gate).

## What Changes

- `genesis::evals::parse_envelope` reads hint commands from **both** channels:
  `data.remediation[].command` (the genesis-native error channel) and the
  top-level `hints[].command` (the success-path channel). Reading is
  tolerant: a hint entry MAY be an object with `command`, or a bare string.
- The eval fixtures stop fabricating envelope JSON. A new helper builds a real
  `genesis::envelope::Envelope::error` (with `ErrorResult.remediation`) and the
  hint-following scenarios assert against it. The success-path fixture
  (`src/evals/tests_e2e.rs:20-22`) is built the same way, so **no** scenario
  fixture is hand-written JSON any more.
- The `envelope` spec states the channel contract explicitly: **error
  remediation lives in `data.remediation`**; the top-level `hints` array is the
  success-path next-step channel and is **deprecated on the error path**
  (readers may still accept it; writers must not be required to emit it).
- The `evals` spec states the reader contract: `parse_envelope` SHALL resolve
  hint commands from both channels, and SHALL NOT report a hint-blindness
  fault when the error channel carries the command.

> **Correction (review finding CORR-002).** The tolerance is **defensive, not a
> migration**: *no* in-fleet producer emits hints on an error path today.
> `Envelope::error` sets `hints: None` (`src/envelope.rs:319`); wai
> (`wai/src/output.rs:12`) and ddL (`dulce-de-leche/src/output.rs:103`) pass
> hints only through `Envelope::success`; kaya's error envelope carries an
> empty `hints` array; sunshine's `Envelope` has no `hints` member at all.
> The reader tolerance therefore covers foreign and older producers (and the
> ordering rule below) — not an existing in-fleet population.

Not breaking for writers: no tool's output shape changes, and no envelope
constructor signature changes. Only readers gain tolerance.

## Capabilities

### New Capabilities

(none)

### Modified Capabilities

- `envelope`: declares the hint/remediation channel contract — which channel
  carries error remediation, which carries success-path next steps, and which
  is deprecated on the error path.
- `evals`: declares the dual-channel hint resolution of `parse_envelope` and
  the requirement that eval fixtures use genesis-built envelopes.

## Impact

- **Affected code**: `src/evals/outcome.rs` (reader), `src/evals/tests_e2e.rs`
  + a new fixture helper (fixtures), `src/envelope.rs` (unchanged — the fix is
  reader-side; the contract is documentation only).
- **Affected specs**: `openspec/specs/envelope/spec.md`,
  `openspec/specs/evals/spec.md` (deltas in this change).
- **Downstream**: `evallerina`'s private dual-channel reader
  (`evallerina/src/envelope.rs:60-84`) becomes redundant — it may delegate to
  `genesis::evals::parse_envelope` in a separate ticket in the evallerina repo.
  `wai` and `dulce-de-leche` need no change (their writers already put
  remediation in `data.remediation`).
- **New tickets**: one bd issue for the evallerina reader consolidation
  (cross-repo, filed at approval).
