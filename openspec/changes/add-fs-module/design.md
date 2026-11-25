# Design: add-fs-module

## Context

See proposal.md — Why. Constraints that shape the approach:

- No new runtime dependency: `std::fs` only. `tempfile` is currently a
  **regular** dependency of genesis (`Cargo.toml:38-41`) even though the
  comment above it says "Test fixtures"; the module must not rely on it at
  runtime, and a task moves it to `[dev-dependencies]` once that is confirmed.
- ddL's implementation is the reference and already carries the platform
  subtleties (`dulce-de-leche/src/dot_ddl/persist.rs:24-33,129`): counter-based
  unique names, `fsync`, no-follow open, `with_sharing_retry` for Windows.
- Several fleet sites write *directories* of staged files (podras
  `verbs/init.rs` stages a directory and hard-links it into place) rather than
  a single file. That shape is out of scope here.

## Goals / Non-Goals

**Goals:**

- One atomic-replace implementation, at ddL's strength, owned by the shared
  crate.
- A guarantee that two concurrent writers to the same target cannot corrupt
  each other (the `wai` failure mode).
- Windows parity with ddL.

**Non-Goals:**

- Directory-level staging (podras `verbs/init.rs`).
- Locking, advisory file locks, or transaction semantics.
- Content hashing, checksums, or integrity verification of the payload.
- Symlink-aware reads beyond the no-follow staging open.

## Decisions

**Port ddL, don't redesign.** The alternative — a fresh, simpler
temp+rename — is what `wai` and `sunshine` already have, and it is the
version that lacks `fsync`, unique names, and the Windows retry. Porting the
strongest copy is both cheaper and safer.

**Unique names via pid + counter.** ddL uses an atomic counter; the shared
helper adds the process id so two *processes* cannot collide (ddL's counter is
per-process). Both together satisfy the "unique staging names" requirement
that `wai`'s fixed `state.yml.tmp` violates.

**`fsync` before rename.** Requirement, not option: a rename that publishes an
empty file after a crash is worse than a failed write. ddL already does it;
`sunshine` and `wai` do not.

**Windows retry lives in the helper.** ddL's `with_sharing_retry` is a
generic retry wrapper; the shared helper takes it as-is rather than
reimplementing the retry policy. The alternative — a POSIX-only helper —
would regress the Windows users ddL explicitly supports (`cmd /C` npm routing
exists for the BatBadBut mitigation). The retry is **bounded** (count and
total duration), so a process holding the target open cannot turn the rename
into a hang.

**Single-file scope.** A directory-staging API (podras `verbs/init.rs`
hard-links a staged file into place) is deferred. The module exposes the
file-level primitive; link-based staging stays in podras.

## Risks / Trade-offs

- [A migration changes durability semantics for a tool that relied on the
  weaker behavior] → Mitigation: the new behavior is strictly stronger
  (flush before rename); tools that depended on a rename without a flush were
  already exposed to a crash window.
- [`fsync` on every write slows hot paths] → Mitigation: `atomic_write` is
  opt-in per call site; tools that write on every keystroke keep their own
  path until measured.
- [The staging file is visible to other processes] → Mitigation: the staging
  name is unique and prefixed; readers of the target never see it. Tools that
  scan directories for stray files (e.g. `pretender` history snapshots) may
  need a cleanup sweep — see the stale-staging requirement in the spec.
- [A hard kill (SIGKILL, power loss) leaves a staging file behind] →
  Mitigation: the staging name carries a recognizable prefix so a stale file
  is identifiable; sweeping it is out of scope for this change and is stated
  as such in the spec.
- [Two tools migrate with different temp-name conventions] → Mitigation: the
  shared helper owns the naming; the migration tickets remove the local
  convention.

## Migration Plan

1. Land `genesis::fs` with tests; no consumer changes (additive, patch bump).
2. Migrate one consumer per repo (separate tickets), each keeping its
   existing error types at the call site.
3. `wai`'s `state.rs` migration is the **bug fix** and lands first in that
   repo: the fixed temp name is a live concurrency hazard.
4. Rollback: each migration is a per-repo revert; the shared module can stay.

## Open Questions

- Whether the module should also expose a "write to a directory" variant
  (podras `verbs/init.rs`) — deferred; it changes no behavior in this change
  and would need its own spec.
