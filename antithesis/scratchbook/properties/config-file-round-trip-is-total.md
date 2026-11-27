# config-file-round-trip-is-total — evidence

Extends the suite-wide entry (`charly/antithesis/scratchbook/properties/genesis.md`,
2026-11-16, commit `a6a5150`); reflects genesis `ddf412d`.

## What led to this property

The 2026-11-16 scan established: `read_from` rejects empty/whitespace content
as `ParseError` while `toml::to_string_pretty` emits `""` for an all-default
config (all fields behind `skip_serializing_if`), so write→read is not total
for a legitimate config type. Both facts re-verified at this commit; the
scratch test `p3_config_roundtrip_all_default` reproduces:

```
P3 written bytes: ""
P3 RESULT: RED — write() output not readable back: config parse error:
  /tmp/genesis-azy-config-…/tool.toml: config file is empty
```

Populated configs roundtrip with byte stability (`p3_config_roundtrip_nonempty`,
GREEN), confirming the failure is specific to the empty-serialization edge.

## New finding: `repo_root` is silently ignored by the default read/write

While wiring the scratch test, a config implemented exactly as the trait doc
describes — `path()` returning the file "relative to `repo_root`"
(`src/config/validation.rs:65-67`) — behaved wrongly:

- `ConfigFile::read`/`write` (`validation.rs:74-89`) do
  `let path = Self::path(repo_root); self.write_to(&path)` — **`repo_root` is
  never joined**. The relative path resolves against the process cwd.
- Every in-tree consumer defensively joins inside `path()`:
  `dont/src/config.rs:330-332` (`repo_root.join(".dont").join("config.toml")`),
  `testaruda/src/config.rs:141-143`, `wai/src/config.rs:185-188` (and its
  `UserConfig` returns an absolute `~/.config` path). So the doc-or-impl
  discrepancy is latent, not active — but any new consumer is one doc-faithful
  implementation away from writing config files into an arbitrary cwd.
- Scratch test `p3a_write_honors_repo_root` (doc-faithful `path()`):
  **RED** — file landed in the process cwd, nothing under `repo_root`.

This is a contract seam property: whichever side is made canonical, a
property test pins it. If `read`/`write` start joining, consumers that join
themselves get `repo_root.join(repo_root.join(x))` — wrong for relative
paths — so the fix direction must migrate all in-tree `path()` impls in the
same release (cheap: three call sites found).

## Check mode, seam, generator

- Check mode: in-process with tempdir.
- Seam: `ConfigFile::read_from`/`write_to` and `read_config_file`
  (`src/config/validation.rs:98-143, 167-175`); `ConfigStore::discover`
  (`src/config/store.rs:29-45`) is order-independent over a `Vec` of
  registered markers (walks `registry.iter()`), so no nondeterminism there.
- Generator: all-default vs. populated struct (Option fields set/unset);
  both `path()` contract readings; assertions on value equality, byte
  stability of write→read→write, and file placement under `repo_root`.

## Consumer version skew

- **Reaches on rebuild** (`^0.14`, locked 0.14.0): `dont`, `wai`,
  `testaruda`, `evallerina`, `dulce-de-leche`.
- **Does not reach** without requirement bumps: `whisper` (locked 0.13.1),
  `gently-cli` (locked 0.13.2), `specodelic` (locked 0.13.2),
  `specodelic-gre-wt` (locked 0.11.1), `bajan` (locked 0.8.3).

The empty-file totality bug bites consumers at their own `path()`s, so
which consumers see relief depends on whether their configs can serialize
to `""` (all-default). `wai`'s `ProjectConfig` and `dont`'s `Config` have
non-optional fields and never serialize empty; the bug is most live for
future consumers whose configs are all-optional — the version-skew answer
there is "whichever consumer adopts the fix's version."

## Open Questions

- Canonical fix direction for the path contract (join in read/write vs.
  doc change) `(needs human input)` — see above.
- Placeholder-vs-accept-empty for all-default structs `(partial:` the
  rejection is genesis's own `read_config_file` choice; `toml 0.8` would
  round-trip the empty table fine`)`.

## Investigation Log

#### Was the 2026-11-16 finding still accurate, and is `ConfigStore::discover` affected?

- Examined: `src/config/validation.rs` (trait defaults, empty rejection),
  `src/config/store.rs` (discover loop), consumer `path()` impls (dont,
  wai, testaruda), scratch execution.
- Found: empty-file RED still reproduces identically; additionally the
  `repo_root` join is missing from the provided methods, contradicting the
  trait doc. `discover` is a plain loop over `registry.iter()` — its order
  is the registry's insertion order, deterministic.
- Not found: any test pinning the relative-path contract (nothing in
  `src/config/tests*.rs` covers where the file lands).
- Conclusion: property kept, extended with the placement invariant.
