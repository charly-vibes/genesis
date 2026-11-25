# Tasks: add-fs-module

Tracked as `genesis-mqt` in beads (base_commit `3bffa47`).

Meter: `just ci` (fmt, clippy, test, docs, build-release, aix-check) and
`ah check`.

## 1. Module (genesis, no new dependencies)

- [ ] 1.1 RED: `tests/fs_atomic_write.rs` — replace a file (content observed
      whole by a concurrent reader); a failure before the rename leaves the
      target byte-for-byte unchanged and removes the staging file; the staging
      file is created in the target's directory, not a system temp dir.
- [ ] 1.2 GREEN: `atomic_write` in `src/fs.rs` — unique staging name
      (`.<name>.<pid>.<counter>.tmp` in the target's directory), write,
      flush, rename, remove staging on every failure path.
- [ ] 1.2a Move `tempfile` from `[dependencies]` to `[dev-dependencies]` in
      `Cargo.toml` once the module confirms it is unused at runtime
      (`Cargo.toml:38-41` currently lists it as a regular dependency under a
      "# Test fixtures" comment).
- [ ] 1.3 RED: `tests/fs_unique_names.rs` — two concurrent writers to the
      same target use distinct staging names; two sequential writes in one
      process do not reuse a name; a leftover staging file never appears in
      the target directory after a successful write.
- [ ] 1.4 GREEN: staging-name generation (pid + atomic counter), with the
      counter shared across calls in the process.
- [ ] 1.5 RED: `tests/fs_windows_retry.rs` — a rename failing with a sharing
      violation is retried and eventually succeeds; a rename failing for
      another reason is not retried and reports the failure; a rename that
      keeps failing with a sharing violation stops after a bounded number of
      attempts (or a bounded total time) and reports the failure.
- [ ] 1.6 GREEN: `with_sharing_retry`-equivalent retry wrapper, ported from
      `dulce-de-leche/src/dot_ddl/persist.rs:129`, with the retry bounded.
- [ ] 1.7 RED: `tests/fs_json.rs` — JSON round-trip via `atomic_write_json` +
      `read_json`; malformed JSON yields a typed error, not a panic.
- [ ] 1.8 GREEN: `read_json` / `atomic_write_json` (serde_json), typed
      `FsError`.
- [ ] 1.9 `lib.rs`: `pub mod fs;`

## 2. Spec (this change)

- [ ] 2.1 Spec delta `specs/fs/spec.md` — atomic replace, unique staging
      names, durability, Windows retry, JSON helpers (done in this change).
- [ ] 2.2 On archive: deploy `openspec/specs/fs/spec.md`.
- [ ] 2.3 `.espectacular/fs/` contract TOMLs mapped to test flags (one per
      scenario).
- [ ] 2.4 `ah check` green.

## 3. Downstream migrations (separate tickets, one per repo — filed at approval)

- [ ] 3.1 wai: `state.rs` — replace the fixed `state.yml.tmp` staging with
      `genesis::fs::atomic_write` (**bug fix**: concurrent writers can publish
      each other's bytes; no `fsync` today).
- [ ] 3.2 dulce-de-leche: `dot_ddl/persist.rs` + `manifest.rs` — call the
      shared helper; keep ddL's SEC-05 no-follow staging semantics.
- [ ] 3.3 kaya: `projection/state.rs` + `publication/io.rs`.
- [ ] 3.4 podras: `write.rs`, `curation.rs`, `roadmap/file.rs` — file staging
      only. `verbs/init.rs` stays local: it stages via **hard-link**, not
      rename, so it is not an `atomic_write` case (see the proposal's
      "Excluded" note).
- [ ] 3.5 sunshine: `stamp.rs` — its local `atomic_write` becomes a thin
      wrapper over the shared helper.
- [ ] 3.6 pretender: `history/events.rs`.
- [ ] 3.7 sad: `baseline.rs`.

## 4. Tidy

- [ ] 4.1 `just ci` green.
- [ ] 4.2 `versions.ddl.toml` **minor** bump entry (new public module).
- [ ] 4.3 CHANGELOG entry naming the durability guarantees.
