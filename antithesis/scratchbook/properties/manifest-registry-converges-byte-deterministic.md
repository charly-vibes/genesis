# manifest-registry-converges-byte-deterministic — evidence

New property (not in the 2026-11-16 catalog); reflects genesis `ddf412d`.

## What led to this property

Ticket genesis-azy names "manifest register/unregister converges and is
byte-deterministic" as a baseline candidate. The manifest is
`.genesis/tools.toml` (`src/discovery.rs`); every genesis-based tool writes
it during `init`, and `wai` reads it to discover tools without hardcoding.

## Why byte-determinism is structurally broken: `HashMap` + TOML tables

`Manifest.tools` is `HashMap<String, ToolEntry>` (`discovery.rs:52-56`).
`write_manifest` runs `toml::to_string_pretty` over it (`discovery.rs:186-193`),
and serde serializes a `HashMap` in **iteration order** — the hash-seeded,
insertion-order-dependent order of `std::collections::HashMap`. Two projects
holding the identical logical tool set therefore serialize in different tool
orders when the maps were built by inserting in different orders, or when
per-process hash seeds differ.

## Execution record (scratch crate `antithesis/scratch-hegel`)

| Test | Outcome |
|---|---|
| `p4a_manifest_register_unregister_converges` (2 registers, 2 unregisters → no manifest file, no `.genesis/` dir; ghost-unregister no-op) | GREEN |
| `p4b_manifest_register_is_byte_deterministic` (same tool set, `wai`-then-`dont` vs. `dont`-then-`wai`) | **RED, and flaky in the way that proves the defect**: five consecutive `cargo test` runs alternated between `6 passed; 6 failed` and `7 passed; 5 failed` — the in-process HashMap comparison sometimes collides into one order, sometimes doesn't |

Observed byte difference (from the failing run's captured output):

```
A: [tools.dont] … [tools.wai] …     ← wai registered first
B: [tools.wai] … [tools.dont] …     ← dont registered first
```

The flakiness is itself evidence: a test that passes on run N and fails on
run N+1 without any input change is the definition of the nondeterminism
this property forbids. A fix is a one-line type change
(`HashMap` → `BTreeMap` in `Manifest`, or sort keys at serialization);
`toml::to_string_pretty` over a `BTreeMap` is insertion-order-independent.
No production change was made in this ticket — that is the recommendation.

## Convergence details (the GREEN half)

- `unregister` on a missing manifest: `Ok(())`, nothing created
  (`discovery.rs:197-201`).
- Unregistering the last tool deletes `tools.toml` **and** removes `.genesis/`
  if empty (`remove_manifest_and_dir`, `discovery.rs:210-218`); the dir
  removal is best-effort (`let _ =`), so a foreign file in `.genesis/`
  correctly prevents deletion.
- `register` after a **corrupt** manifest: `read_manifest` → `None` on parse
  error, `register` starts from `Manifest::default()` and overwrites the
  file — every other tool's entry is silently dropped. Recorded as an open
  question (fresh-start semantics vs. data loss).

## Check mode, seam, generator

- Check mode: in-process with tempdirs for the convergence half; the
  byte-determinism half is genuinely **cross-process** (hash seeds), so the
  strongest form of the test compares manifests produced by separate
  processes or pins the hasher. An in-process two-map comparison catches it
  only stochastically — acceptable as a tripwire, insufficient as a proof.
  This is the one property of the four where a Hegel single-process
  generator adds little and a plain deterministic test (or `BTreeMap`
  refactor + test) is the right instrument.
- Seam: `discovery::register`/`unregister`/`write_manifest`
  (`src/discovery.rs:154-218`).
- Generator: tool set from a fixed name pool × detector types
  (directory/file), registration order as a drawn permutation.

## Consumer version skew

Manifest writers/readers:

- **Reaches on rebuild** (`^0.14`, locked 0.14.0): `wai` (primary reader —
  `discovery` is documented as "orchestration tools like wai"), `dont`,
  `testaruda`, `evallerina`, `dulce-de-leche`.
- **Does not reach** without bumps: `whisper` (locked 0.13.1),
  `gently-cli` (locked 0.13.2), `specodelic` (locked 0.13.2),
  `specodelic-gre-wt` (locked 0.11.1), `bajan` (locked 0.8.3).

Because the file's bytes depend on which genesis version each tool was
built against, mixed-version fleets (e.g. `wai` on 0.14 + `specodelic` on
0.13) writing the same manifest is exactly the scenario where determinism
matters most — and the skew means only part of the fleet would stabilize
its output from a 0.14-line fix. `kaya` has no genesis dependency (its
`.genesis` mention is a directory marker), so it is unaffected either way.

## Open Questions

- Is byte-determinism required, or semantic equality only? `(needs human
  input)` — the ticket asserts byte-deterministic; nothing in-tree pins it.
- Corrupt-manifest `register` dropping sibling entries: intended or hazard?
  `(partial: behavior confirmed in code; severity is a product call)`

## Investigation Log

#### Does register/unregister actually converge?

- Examined: `src/discovery.rs:154-218`, executed `p4a` in the scratch crate.
- Found: full convergence including manifest+dir removal and no-op ghost
  unregister.
- Not found: any path that leaves a partial manifest (writes are single
  `std::fs::write` calls; no temp-file/rename dance, so a crash mid-write
  can leave a truncated manifest — outside this property's scope, noted for
  future durability work).
- Conclusion: convergence GREEN; nondeterminism is the live defect.
