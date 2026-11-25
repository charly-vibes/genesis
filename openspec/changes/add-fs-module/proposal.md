# Proposal: add-fs-module

Tracked as `genesis-mqt` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`CORR-001`, `CORR-003`, `CORR-004`, `CORR-005`, `DRAFT-002`, `DRAFT-003`,
`CLAR-001`, `CLAR-002`, `CLAR-003`, `EDGE-002`, `EDGE-003`, `EDGE-004`,
`EXCL-001`–`EXCL-004`.

Evidence: 12 atomic-write sites in 7 tools, each re-checked in this session:
`dulce-de-leche/src/dot_ddl/persist.rs` (+`manifest.rs`), `kaya/src/projection/state.rs`
(+`publication/io.rs`), `podras/src/write.rs`, `podras/src/curation.rs`,
`podras/src/roadmap/file.rs`, `sunshine/src/stamp.rs`,
`pretender/pretender/src/history/events.rs`, `sad/sad/src/baseline.rs`,
`wai/src/state.rs`. `podras/src/verbs/init.rs` stages via **hard-link**, not
rename, and is excluded (see below).

Verification: the strongest-implementation claim was independently checked
(confidence 0.88 and 0.92, above the 0.8 gate).

Scope: atomic file writes across the fleet — 18 Rust CLIs scanned, 12 implementation sites in 7 tools.

Depends on: nothing. Blocked by: nothing. Unblocks: `add-fs-module`
migrations in 6 other repos.

Capability path: `fs` (new); module: `genesis::fs`.

Version bump: **minor** — a new public module.

Downstream repos: 6 (dulce-de-leche, kaya, podras, sunshine, pretender, sad)
plus `wai` — one ticket each; `wai`'s is a **bug fix**, not a de-duplication.

## Why

Twelve hand-rolled atomic-write sites exist across seven fleet tools, with four
incompatible temp-naming schemes and two different levels of durability:

- `dulce-de-leche/src/dot_ddl/persist.rs` — counter-based unique name, `fsync`,
  no-follow open (SEC-05), Windows sharing-violation retry (`with_sharing_retry`,
  persist.rs:129);
- `dulce-de-leche/src/manifest.rs`, `kaya/src/projection/state.rs`,
  `kaya/src/publication/io.rs`, `podras/src/{write,curation,verbs/init,roadmap/file}.rs`,
  `sunshine/src/stamp.rs`, `pretender/pretender/src/history/events.rs`,
  `sad/sad/src/baseline.rs` — temp sibling + rename, each with its own naming;
- `wai/src/state.rs:122-131` — **fixed** temp name `state.yml.tmp` (no pid, no
  counter) and no `fsync`: two concurrent `wai` processes write the same path,
  so one can publish the other's bytes and the second rename fails with ENOENT.

The strongest implementation of the pattern is in ddL, not in the shared crate.
That is the inversion this change fixes: the shared crate should own the best
copy, and the fleet should converge on it.

Boundary rule: two or more tools need it — seven do, by a wide margin.

## What Changes

- New `genesis::fs` module with:
  - `atomic_write(path, bytes)` — stages a uniquely-named sibling temp file,
    `fsync`s it, renames it over the target, removes the temp file on every
    failure path. The temp file lives in the target's directory so the rename
    stays on one filesystem.
  - `atomic_write_json` / `read_json` — JSON helpers over `atomic_write`.
  - Documented failure semantics: the prior target is left byte-for-byte
    intact on any failure before the rename.
  - Windows parity: the rename step retries transient sharing violations
    (ddL's `with_sharing_retry` behavior), because `std::fs::rename` on Windows
    uses `MOVEFILE_REPLACE_EXISTING` and can fail with a sharing violation
    while another process holds the file open.
- The reference implementation is ddL's `stage_manifest` + `with_sharing_retry`,
  ported rather than re-invented.
- Downstream migrations of the twelve sites (one ticket per repo), including
  the `wai` fix for the fixed-name temp file, which is a **bug fix**, not
  merely a de-duplication.
- **Excluded**: `podras/src/verbs/init.rs` stages a file into place with a
  **hard link**, not a rename — a different mechanism, not an `atomic_write`
  case; it stays local until a separate proposal covers link-based staging.

### Public API (sketch)

```rust
use genesis::fs::{atomic_write, atomic_write_json, read_json, FsError};

atomic_write(&path, &bytes)?;              // stage → flush → rename
atomic_write_json(&path, &value)?;         // serialize + atomic_write
let cfg: MyConfig = read_json(&path)?;     // typed error on malformed JSON
```

## Capabilities

### New Capabilities

- `fs`: shared file-writing primitives — atomic replace, JSON read/write, and
  the durability and platform guarantees that go with them.

### Modified Capabilities

(none)

## Impact

- **Affected code**: `src/fs.rs` (new), `src/lib.rs` (module decl).
  The module adds **no new dependency**: it uses `std::fs` only, and does not
  rely on `tempfile` at runtime. Note that `tempfile` is currently a
  **regular** dependency (`Cargo.toml:38-41` — `# Test fixtures` /
  `tempfile = "3"` under `[dependencies]`, not `[dev-dependencies]`); a task
  moves it to `[dev-dependencies]` if the module confirms it is unused at
  runtime.
- **Affected specs**: new `openspec/specs/fs/spec.md`.
- **Downstream migrations** (separate tickets, one per repo):
  - `dulce-de-leche` — `dot_ddl/persist.rs`, `manifest.rs`
  - `kaya` — `projection/state.rs`, `publication/io.rs`
  - `podras` — `write.rs`, `curation.rs`, `roadmap/file.rs`
  - `sunshine` — `stamp.rs` (its `atomic_write` becomes a thin wrapper)
  - `pretender` — `history/events.rs`
  - `sad` — `baseline.rs`
  - `wai` — `state.rs` (**bug**: fixed temp name, no fsync)
- **New tickets**: one bd issue per downstream migration; the `wai` one is
  filed as a bug, not a refactor.
