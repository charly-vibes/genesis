---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: /var/home/sasha/para/areas/dev/gh/charly/genesis/antithesis/scratchbook/property-catalog.md
    why: The catalog this file maps relationships for (copied external_references per scratchbook-setup).
---

# Property relationships — genesis

Derivative of [property-catalog.md](property-catalog.md); slugs are catalog IDs.

## Cluster: user-file rewrite safety

`managed-block-markers-stay-unique-and-idempotent` +
`config-file-round-trip-is-total`

Both properties guard irreversible rewrites of user-owned files with
whole-file truncate+write and no temp/rename. Their failure modes compound:
the config placement defect (`repo_root` ignored) writes to the cwd the same
way a bad managed-block splice corrupts in place — both are silent from the
caller's perspective (`write`/`inject` return `Ok`). The clusters' shared
future property is write durability (temp-file+rename), currently out of
scope (see sut-analysis § failure-prone areas). Dominance: if a
durability property is ever added, it dominates both current entries.

## Cluster: documented-contract vs. implemented-contract drift

`error-envelope-exposes-its-fix` + `config-file-round-trip-is-total`

Both found a doc-level invariant the implementation doesn't fully provide
(Invariant 3.2.5 vs. derived `Deserialize`; "relative to `repo_root`" vs.
never joining). The pattern is the ticket's most reusable finding: in this
crate, properties should be written against observed behavior and flag the
doc, because the docs overpromise. Dominance: none — each pins a different
seam; fixing one does not constrain the other.

## Cluster: serialization determinism

`manifest-registry-converges-byte-deterministic` (singleton)

The only true nondeterminism found (HashMap iteration order in TOML
output). Weak link to the config cluster via `toml::to_string_pretty`:
config serialization is deterministic only because structs serialize fields
in declaration order — a future switch to maps in configs would import the
same defect. Noting the connection guards against that drift.

## Independence note

`error-envelope-exposes-its-fix` shares no state with the other three
(pure construct/parse, no filesystem) and can run in any order; the other
three all touch tempdirs and should not share a directory. No dominance or
interference between clusters otherwise.
