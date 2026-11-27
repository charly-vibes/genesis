---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: https://hegel.dev/
    why: Target PBT engine (hegeltest 0.52.7) whose fit this analysis informs.
  - path: /var/home/sasha/para/areas/dev/gh/charly/antithesis/scratchbook/sut-analysis.md
    why: Suite-wide SUT analysis from the 2026-11-16 pass; genesis section extended here, not duplicated.
---

# SUT analysis — genesis (crate `genesis-vibes` 0.14.0)

Extends the suite-wide SUT analysis in the charly superproject scratchbook
(`charly/antithesis/scratchbook/sut-analysis.md`) with what this ticket's
four seams need. Genesis is a library-only shared crate ("Shared crate for
cross-cutting CLI/AIX/self-healing infrastructure"); it has no binary and no
deployment surface — all properties are in-process.

## Architecture in scope

| Seam | Module | Shape | Concurrency |
|---|---|---|---|
| JSON envelope | `src/envelope.rs`, `src/envelope/{notice,outcome,receipt}.rs` | Construct-once types + lenient JSON reader; process-global `OnceLock` author | None in the checked paths |
| Managed blocks | `src/managed_block.rs`, `src/managed_block/{splice,provenance}.rs` | Read file → locate span → splice → write whole file (truncate+write, no temp/rename) | None; callers are single-threaded CLIs |
| Config | `src/config.rs`, `src/config/{validation,registry,store,error}.rs` | Trait-defaulted serde read/write over explicit paths; `ConfigStore` walks a registry `Vec` | None |
| Manifest discovery | `src/discovery.rs` | Read-or-default TOML manifest, `HashMap` tool table, whole-file write | None |

## State and failure-prone areas

- **Whole-file truncate+write everywhere** (`managed_block.rs:202,267,283`,
  `config/validation.rs:141`, `discovery.rs:192`): a crash mid-write can
  leave truncated user files. No temp-file+rename anywhere in the four
  seams. Out of this ticket's property scope (would need crash-fault
  injection); recorded for future work.
- **Marker-adjacent text handling** is the richest failure surface found:
  `block_span`'s first-START/END-after-START guard fixes the 2026-11-16
  defect but introduces a prepend fallback that duplicates markers on
  quote-bearing text, and a splice that can consume user text.
- **Contract drift between docs and impls** is a repeated pattern: the
  `ConfigFile::path` doc ("relative to `repo_root`") contradicts the default
  `read`/`write` (never join); the envelope's Invariant 3.2.5 doc claims a
  type-level guarantee the derived `Deserialize` doesn't provide. Properties
  should pin the *implemented* contract and flag the doc mismatch.
- **Hash-order leakage**: the only true nondeterminism found (`Manifest`
  serialization), and it is cross-process — invisible to single-run tests.

## Existing test coverage (what the SUT already pins)

- `src/managed_block/tests/splice.rs`: update/prepend/no-duplicate markers,
  plus the end-before-start regression added after the 2026-11-16 scan
  (asserts a stray quoted END survives — 2 END markers — as intended).
- `src/envelope/tests/outcome.rs`: preamble tolerance, both hint channels,
  value-direct parsing, missing-`ok` rejection.
- `src/config/tests*.rs`: happy-path read/write/validate; **no** placement
  test for where `write(repo_root)` lands the file, **no** all-default
  roundtrip.
- `src/discovery/tests.rs`: register/scan/unregister basics.

No Antithesis SDK assertions exist anywhere in the codebase (see
[existing-assertions.md](existing-assertions.md)).

## Provenance caveat

The working tree during this run carried unrelated uncommitted `fs` work by
another agent (`src/fs.rs`, `src/lib.rs` module hookup, `tests/fs_*.rs`).
None of the four seams above imports or is imported by the new `fs` module;
scratch-crate results are attributed to `ddf412d` + that inert addition.
