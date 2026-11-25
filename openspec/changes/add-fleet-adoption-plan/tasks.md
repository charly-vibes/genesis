# Tasks: add-fleet-adoption-plan

Tracked as `genesis-svt` in beads (base_commit `3bffa47`).

Meter: none — this change ships no code. Completion is the ticket set
existing in the owning repos' trackers.

## 1. New adopters (4 repos — no genesis dependency today)

- [ ] 1.1 kaya: adopt `genesis::envelope` + `genesis::cli`; keep its own
      `envelope.rs` as a thin wrapper so the output shape is unchanged.
- [ ] 1.2 sunshine: adopt `genesis::envelope`; map its kinds (`render`,
      `audit`) onto genesis kinds and update any consumer matching on those
      values.
- [ ] 1.3 funkalization: adopt `genesis::envelope` + `genesis::cli`; its own
      `Error` type stays as the domain error.
- [ ] 1.4 wild: wrap the `v1` envelope as `data` inside a genesis envelope
      (the v1 payload contract is unchanged).

## 2. Stale pins (2 repos)

- [ ] 2.1 bajan: bump `genesis-vibes` 0.8 → 0.13.x and rewrite the
      `Envelope::success` call sites in `bajan/src/cli.rs` (the signature
      gained arguments — a breaking change, not a one-line pin edit).
- [ ] 2.2 evallerina: move off the frozen git tag `v0.12.3` to a 0.13.x
      release; re-check any 0.12-only API it still calls.
- [ ] 2.3 testeruda: **re-measure** adoption (the source tree was unreadable
      during the scan) before any adoption claim is recorded.

## 3. Git hooks (3 repos)

- [ ] 3.1 wai: migrate `wai/src/commands/way/hooks.rs` and
      `wai/src/commands/way/guide/hooks.rs` onto `genesis::git_hooks`,
      preserving the deliberate `--local`-only scope semantics (beads
      `genesis-orq`).
- [ ] 3.2 dulce-de-leche: migrate `dulce-de-leche/src/gates/lefthook.rs`
      onto `genesis::git_hooks`.
- [ ] 3.3 pretender: migrate `pretender/pretender/src/cli/hooks.rs` onto
      `genesis::git_hooks`, preserving scope semantics (beads `genesis-q4k`).

## 4. Feedback rollout (9 repos)

- [ ] 4.1 Adopt `genesis::feedback` in kaya, evallerina, sunshine, wild,
      bajan, sad, funkalization, podras, testeruda (beads `genesis-dvf`
      covers testeruda).

## 5. Partial adopters (4 repos — fill the missing modules)

- [ ] 5.1 whisper: envelope, cli, config, status, aix.
- [ ] 5.2 podras: doctor, config, feedback, guide, status.
- [ ] 5.3 sad: guide, config, doctor, feedback, cli.
- [ ] 5.4 gently: envelope, cli, config, feedback, status.

## 6. Index

- [ ] 6.1 Each row above is filed as a ticket in the **owning** repo's
      tracker, with its scope and blast radius from the proposal's table.
- [ ] 6.2 `document-fleet-adoption-review` links this index from the docs
      page.
- [ ] 6.3 No `versions.ddl.toml` entry (no genesis code changes here).
