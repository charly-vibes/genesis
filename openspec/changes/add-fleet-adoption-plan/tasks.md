# Tasks: add-fleet-adoption-plan

Tracked as `genesis-svt` in beads (base_commit `3bffa47`).

Meter: none — this change ships no code. Completion is the ticket set
existing in the owning repos' trackers.

## 1. New adopters (4 repos — no genesis dependency today)

- [x] 1.1 kaya: adopt `genesis::envelope` + `genesis::cli`; keep its own
      `envelope.rs` as a thin wrapper so the output shape is unchanged.
      → `kaya-haz` (P1).
- [x] 1.2 sunshine: adopt `genesis::envelope`; map its kinds (`render`,
      `audit`) onto genesis kinds and update any consumer matching on those
      values. → `sun-eqw` (P1).
- [x] 1.3 funkalization: adopt `genesis::envelope` + `genesis::cli`; its own
      `Error` type stays as the domain error. → `funkalization-62b` (P1).
- [x] 1.4 wild: wrap the `v1` envelope as `data` inside a genesis envelope
      (the v1 payload contract is unchanged). → `wild-lof` (P1).

## 2. Stale pins (2 repos)

- [x] 2.1 bajan: bump `genesis-vibes` 0.8 → 0.13.x and rewrite the
      `Envelope::success` call sites in `bajan/src/cli.rs` (the signature
      gained arguments — a breaking change, not a one-line pin edit).
      → `bajan-rai` (P2; still on 0.8 as of 2026-11-27, retargeted to 0.14.x).
- [x] 2.2 evallerina: move off the frozen git tag `v0.12.3` to a 0.13.x
      release; re-check any 0.12-only API it still calls.
      → DONE (verified 2026-11-27): on crates.io `genesis-vibes` 0.14
      (genesis-0zh, commit 8f14e2c).
- [x] 2.3 testeruda: **re-measure** adoption (the source tree was unreadable
      during the scan) before any adoption claim is recorded.
      → BLOCKED as of 2026-11-27: repo `/var/home/sasha/para/areas/dev/gh/charly/testeruda`
      does not exist. Re-measure once the tree is readable.

## 3. Git hooks (3 repos)

- [x] 3.1 wai: migrate `wai/src/commands/way/hooks.rs` and
      `wai/src/commands/way/guide/hooks.rs` onto `genesis::git_hooks`,
      preserving the deliberate `--local`-only scope semantics (beads
      `genesis-orq`).
      → DONE (verified 2026-11-27): `genesis-orq` closed, wai b8ab821.
- [x] 3.2 dulce-de-leche: migrate `dulce-de-leche/src/gates/lefthook.rs`
      onto `genesis::git_hooks`. → `DDL-gzak` (P2; still hand-rolls
      managed-block wiring as of 2026-11-27).
- [x] 3.3 pretender: migrate `pretender/pretender/src/cli/hooks.rs` onto
      `genesis::git_hooks`, preserving scope semantics (beads `genesis-q4k`).
      → Already tracked: `genesis-q4k` (open).

## 4. Feedback rollout (9 repos)

- [x] 4.1 Adopt `genesis::feedback` in kaya, evallerina, sunshine, wild,
      bajan, sad, funkalization, podras, testeruda (beads `genesis-dvf`
      covers testeruda).
      → Filed 2026-11-27: `kaya-bov`, `sun-6oz`, `funkalization-rjb`,
      `wild-8g8`, `bajan-rai` (folded into the pin-bump ticket), 
      `evallerina-e4r`. For sad/podras/gently the feedback module is part of
      the 5.3/5.2/5.4 tickets below (no duplicate ticket). testeruda: covered
      by `genesis-dvf` (open).

## 5. Partial adopters (4 repos — fill the missing modules)

- [x] 5.1 whisper: envelope, cli, config, status, aix. → `whisper-g6p` (P2).
- [x] 5.2 podras: doctor, config, feedback, guide, status. → `podras-yfx` (P2).
- [x] 5.3 sad: guide, config, doctor, feedback, cli. → `sad-q3n` (P2).
- [x] 5.4 gently: envelope, cli, config, feedback, status. → `gently-38w` (P2).

## 6. Index

- [x] 6.1 Each row above is filed as a ticket in the **owning** repo's
      tracker, with its scope and blast radius from the proposal's table.
      → Done 2026-11-27: 16 tickets filed (see per-row IDs above).
- [ ] 6.2 `document-fleet-adoption-review` links this index from the docs
      page.
- [ ] 6.3 No `versions.ddl.toml` entry (no genesis code changes here).
