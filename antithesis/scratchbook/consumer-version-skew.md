---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: https://hegel.dev/
    why: Engine whose fit per property this note supports.
  - path: /var/home/sasha/para/areas/dev/gh/charly/reports/hegel-eval-2026-11-16/README.md
    why: Prior hands-on engine verification this note builds on.
---

# Consumer version skew — which genesis fix reaches which tool

Every property in this catalog proposes (eventually) a fix or a test in
genesis `src/`. Genesis is consumed by suite tools pinning different version
requirements, so a fix merged and released as the next `genesis-vibes`
version reaches different tools at different times — possibly never, absent
a requirement bump.

## Pin inventory (requirements from each consumer's `Cargo.toml`, resolved versions from each consumer's `Cargo.lock`, checked 2026-11-27)

| Consumer | Requirement | Locked | A next-release fix… |
|---|---|---|---|
| `dont` | `0.14.0` (caret) | 0.14.0 | reaches on next `cargo update` |
| `wai` | `0.14` (+`update-check`) | 0.14.0 | reaches on next `cargo update` |
| `testaruda` | `0.14` | 0.14.0 | reaches on next `cargo update` |
| `evallerina` | `0.14` | 0.14.0 | reaches on next `cargo update` |
| `dulce-de-leche` | `0.14` | 0.14.0 | reaches on next `cargo update` |
| `specodelic` | `0.13` | 0.13.2 | does **not** reach (0.14 excluded by `^0.13`) |
| `gently-cli` | `0.13.2` | 0.13.2 | does **not** reach |
| `whisper` | `0.13.1` | 0.13.1 | does **not** reach |
| `specodelic-gre-wt` | `0.11` | 0.11.1 | does **not** reach |
| `bajan` | `0.8` | 0.8.3 | does **not** reach |
| `kaya` | — | — | no genesis dependency (unaffected) |

The ticket's "0.13.1, 0.14.0 pinned" is the boundary: the 0.14 line (five
consumers) vs. the ≤0.13 stragglers (five consumers on four different
lines, the oldest at 0.8.3).

## Consequences for this ticket's properties

1. **A 0.14-line fix reaches only half the fleet.** Any property whose fix
   lands in genesis 0.14.x (e.g. `BTreeMap` for the manifest, empty-config
   totality, deserialize-side Invariant 3.2.5) protects `dont`, `wai`,
   `testaruda`, `evallerina`, `dulce-de-leche` on their next update and
   leaves `whisper`, `gently`, `specodelic` (both), `bajan` on the old
   behavior until they bump. Bumping `^0.13 → ^0.14` is a semver-compatible
   consumer-side edit but is a separate decision per tool (the 0.8/0.11
   consumers are two-plus major-minor cycles behind).
2. **The managed-block module is forked.** `src/managed_block.rs` is
   documented as ported from `wai/src/managed_block.rs`, and wai (and dont,
   per the envelope's "Changes from dont" header) historically forked whole
   modules before consolidating into genesis. A fix to `block_span`/
   `inject_into_existing` in genesis does not touch any still-forked copy;
   the per-property evidence files note where a port-back check is needed.
3. **Manifest determinism is a fleet-wide property with per-tool fixes.**
   The manifest bytes depend on the writing tool's genesis version, so even
   a 0.14-line fix yields deterministic-per-version, not deterministic-
   fleet, output while 0.13-era tools still write. The property's
   byte-stability half should therefore be tested *within* one genesis
   version (its true guarantee) and the fleet uniformity treated as a
   release-engineering concern, not a code invariant.
4. **Test (not fix) placement is skew-free.** The Hegel tests recommended
   in [evaluation/synthesis.md](evaluation/synthesis.md) live in genesis's
   own dev-dependencies against `path = ../..`-style or latest source —
   they always exercise current HEAD regardless of consumer pins.
