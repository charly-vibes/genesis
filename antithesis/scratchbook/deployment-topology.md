---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: /var/home/sasha/para/areas/dev/gh/charly/antithesis/scratchbook/deployment-topology.md
    why: Suite-wide topology decision from the 2026-11-16 pass; extended here.
---

# Deployment topology — genesis

Same conclusion as the suite-wide pass, restated for this crate: **no
Antithesis deployment**. Genesis is a library-only crate with no binary, no
network surface, and no persistent state beyond files its consumers own. All
four cataloged properties are local, single-process checks:

- `error-envelope-exposes-its-fix` — pure construct/serialize/parse.
- `managed-block-markers-stay-unique-and-idempotent` — tempdir file text.
- `config-file-round-trip-is-total` — tempdir file, path placement.
- `manifest-registry-converges-byte-deterministic` — tempdirs; the
  byte-determinism half is cross-process by nature (hash seeds), which a
  plain multi-process test or a `BTreeMap` refactor covers without any
  container topology.

Antithesis's strengths (fault injection, concurrency exploration, multi-node
timelines) find no purchase here: no concurrency in the four seams, no
crash-recovery machinery to fault (writes are truncate+write with no
recovery path), no distribution. The execution engine that fits is Hegel
in-process (see the recommendation in
[evaluation/synthesis.md](evaluation/synthesis.md)).

If a future ticket adds crash-durability machinery to managed-block or
config writes (temp-file+rename), revisit: at that point fault injection
around the write boundary becomes meaningful and a minimal one-container
Antithesis topology (genesis test template + fault profiles on `fs` calls)
would be justified.
