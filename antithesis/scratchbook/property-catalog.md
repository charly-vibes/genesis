---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: https://hegel.dev/
    why: Property-based testing engine the ticket targets (hegeltest 0.52.7); candidate executor for these properties.
  - path: https://github.com/antithesishq/antithesis-skills
    why: Source of the antithesis-research method and artifact formats used for this run.
  - path: /var/home/sasha/para/areas/dev/gh/charly/antithesis/scratchbook/property-catalog.md
    why: Prior suite-wide catalog (2026-11-16); this run extends its three genesis entries rather than duplicating them.
  - path: /var/home/sasha/para/areas/dev/gh/charly/reports/hegel-eval-2026-11-16/README.md
    why: Prior hands-on Hegel evaluation; adopted verdict and hands-on facts (build times, static-engine, shrink quality).
---

# Property catalog — genesis (crate `genesis-vibes`)

## Scope

This catalog covers the four seams named in ticket genesis-azy: the shared
JSON envelope (`src/envelope.rs`), managed blocks (`src/managed_block*`),
config read/write (`src/config*`), and manifest discovery (`src/discovery.rs`,
the `.genesis/tools.toml` manifest). It extends the suite-wide catalog of
2026-11-16: the three prior `genesis/...` entries are revised in place (one
was fixed since that scan, two are overturned in part) and one new entry is
added for the manifest.

Genesis is a shared library consumed by most other suite CLIs, so a defect
here propagates; the flip side is that every fix is gated by consumer version
skew (see [consumer-version-skew.md](consumer-version-skew.md)).

**Verification status:** every "predicted red/green" below was **executed** on
this commit, not just predicted. The scratch crate
[`antithesis/scratch-hegel/`](../scratch-hegel/) compiles genesis as a path
dependency and runs plain-cargo checks for all four seams plus one
`#[hegel::test]` run for the managed-block property. Scratchbook provenance
caveat: the working tree carried unrelated uncommitted `fs` work
(`src/fs.rs`, `src/lib.rs` hook-up) by another agent; none of it touches the
four modules analyzed here.

## Categories

- **Protocol contracts** — the envelope is the single output format for every
  consumer CLI; a lie here misleads every machine reader at once.
- **File rewrite safety** — managed blocks and configs rewrite user-owned
  files; corruption is user-visible and crosses tool boundaries.
- **Manifest state machine** — register/unregister must converge and produce
  byte-identical output for identical logical state.

---

### error-envelope-exposes-its-fix — a named remediation is never invisible to consumers

| | |
|---|---|
| **Type** | Safety (protocol contract) |
| **Property** | For every error envelope any construction path can produce, the commands named in `data.remediation[]` appear in `parse_envelope_value`'s `hint_commands`, and an error envelope that names no fix reports zero commands (never phantom commands). |
| **Invariant** | `Always`: serialize the envelope to JSON, call `parse_envelope_value`, and check that every `data.remediation[].command` is present in `hint_commands`. A second `Always` (currently violated) applies to the type-level invariant: no `Envelope<ErrorResult>` value with an empty `remediation` can exist by any path — the constructor (`ErrorResult::new`) enforces Invariant 3.2.5, the derived `Deserialize` does not, so `Unreachable` on the ingress path fails today. |
| **Check mode** | Pure in-process: construct/parse, no filesystem, no subprocess. |
| **Seam** | `ErrorResult::new` / `Envelope::error` (`src/envelope.rs:130-160, 274-296`) and the lenient reader `parse_envelope_value` (`src/envelope/outcome.rs:76-93`). The reader is the consumer-side channel: it unions `data.remediation[]` (first) with legacy top-level `hints`. |
| **Generator sketch** | Draw an `ErrorResult` (code, message, 0–3 remediation entries, each with/without a `command`), plus an entry-mode switch: constructor-built vs. `serde_json` round-trip vs. raw JSON with fields missing. Assert the reader exposes every named command and that `malformed_hints` counts entries without commands. |
| **Predicted red/green** | **GREEN** on the constructor→reader path (fixed since 2026-11-16: the reader now unions both channels; previously `Envelope::error` set `hints: None` and the reader read only `hints`). **RED** on ingress: `serde_json::from_str::<Envelope<ErrorResult>>` accepts `"remediation": []` — the derived `Deserialize` bypasses Invariant 3.2.5 (verified; note the test fixture must include `spec_ref`/`entity_id`/`unmet_clauses`, which have no serde defaults). Reader reports zero commands faithfully, so consumers see an error that names no fix — the exact state Invariant 3.2.5 forbids. |
| **Why It Matters** | Every CLI's machine-readable failure path flows through this type; the evals loop scores agents on `hint_commands`. An error that names no fix is a dead end for automated recovery. |

**Open Questions:**

- Should Invariant 3.2.5 be enforced on ingress (custom `Deserialize` or
  try-construct-and-reject) or is the constructor the only enforcement point?
  `(partial: derived Deserialize demonstrably accepts empty remediation; no
  consumer relies on parsing hostile envelopes was checked for dont/wai/
  testaruda's use of read_from, but envelope-ingress-from-disk paths in
  consumers were not enumerated)`

Evidence: [properties/error-envelope-exposes-its-fix.md](properties/error-envelope-exposes-its-fix.md)

---

### managed-block-markers-stay-unique-and-idempotent — injection never multiplies or misorders markers

| | |
|---|---|
| **Type** | Safety |
| **Property** | After `inject` on a file with **any** pre-existing text, the file contains at most one START marker, at most one END marker, any START precedes any END, no user text between a quoted marker and the real block is destroyed, and re-injecting identical content is byte-identical. (Refined from the ticket's "exactly one ordered pair": the existing regression test `test_inject_end_marker_before_start_is_not_corrupted` codifies a stray quoted END surviving as correct behavior, so exactly-one-pair is not the documented contract today — see evidence file.) |
| **Invariant** | `Always`: marker counts and ordering over arbitrary generated file text; `Always`: inject is a fixpoint (second inject is byte-identical). |
| **Check mode** | In-process with tempdir. |
| **Seam** | `BlockInjector::inject` / `inject_into_existing` (`src/managed_block.rs:249-270`) over `splice::block_span` (`src/managed_block/splice.rs:22-37`) and `prepend_block` (`splice.rs:9-15`). |
| **Generator sketch** | Generate a file as a sequence of up to ~8 lines drawn from a pool: prose, heading, raw START, raw END, prose quoting START, blank. Inject fixed content; assert marker counts/order, survival of non-marker lines, then re-inject and compare bytes. Executed under `#[hegel::test]` with `gs::vecs(gs::integers::<usize>().max_value(5)).max_size(8)`. |
| **Predicted red/green** | **RED** — three distinct failure modes found at this commit (two new since the 2026-11-16 scan): (1) a file whose only START occurrence is prose (quoting) with no END after it gets a full block *prepended*, yielding **two START markers**; (2) a quoted START occurring before the file's real block makes `block_span` splice from the quote to the real block's END, **destroying all user text in between** (verified: "# Real section / precious user notes" deleted); (3) a stray END before the real block survives injection, leaving the file with two END markers — which the existing test asserts as intended, so this mode is a semantics decision, not a bug claim. **GREEN**: idempotency on clean files, END-before-START no longer duplicates the tail (the guard found by the prior scan is in place and Hegel shrinks mode (1) to the minimal case `picks = [2]` — a file containing only a START marker — with a working `reproduce_failure` blob). |
| **Why It Matters** | `AGENTS.md`-class files are rewritten by several tools in the suite; a duplicated or corrupted managed block is user-visible damage that silently crosses tool boundaries. |

**Open Questions:**

- Should a file containing a bare START occurrence (quoted or stray) get the
  block prepended, or should `inject` treat a start-without-end span as the
  block's opening and close it? `(needs human input)` — prepending is today's
  behavior and it is what duplicates the marker.
- Is the stray-END-survives outcome (2 END markers) the intended long-term
  contract, or should inject normalize to one pair? `(needs human input)` —
  the existing test pins it as intended.

Evidence: [properties/managed-block-markers-stay-unique-and-idempotent.md](properties/managed-block-markers-stay-unique-and-idempotent.md)

---

### config-file-round-trip-is-total — write/read never loses or rejects its own output

| | |
|---|---|
| **Type** | Safety |
| **Property** | For any config struct implementing `ConfigFile`: `write` then `read` returns an equal value — including all-default structs — and write→read→write is byte-identical; additionally `write(repo_root)` places the file under `repo_root` as the trait doc says `path()` is "relative to `repo_root`". |
| **Invariant** | `Always`: value equality after roundtrip; `Always`: byte stability over write→read→write; `Always`: the written file is inside the `repo_root` passed to `write`. |
| **Check mode** | In-process with tempdir. |
| **Seam** | `ConfigFile::read_from`/`write_to` (`src/config/validation.rs:98-143`) and `read_config_file`'s empty-content rejection (`validation.rs:167-175`); consumer-side `path()` impls (dont, wai, testaruda). |
| **Generator sketch** | Draw an all-default vs. populated config struct (Option fields set/unset); write, read, compare; rewrite and compare bytes. Two contract readings: `path()` joining `repo_root` (what every current consumer does) vs. returning the bare relative path the doc describes. |
| **Predicted red/green** | **RED** on totality: an all-default struct (all fields `skip_serializing_if`) serializes via `toml::to_string_pretty` to `""`; the write succeeds, and `read_from` rejects the empty file with `ParseError: config file is empty` (verified). **RED** on the documented path contract: `read`/`write` call `Self::path(repo_root)` and **never join `repo_root`**; a consumer following the doc writes into the process cwd (verified). Every current consumer defensively joins inside `path()` (dont `config.rs:331`, testaruda `config.rs:142`, wai `config.rs:186`), so the doc or the impl is wrong — a new consumer hits this. **GREEN** for populated configs: value roundtrip and byte stability hold. |
| **Why It Matters** | A downstream tool with an all-default config writes a file it cannot read back; the cwd leak writes config files into whatever directory the tool was launched from. |

**Open Questions:**

- Which side of the path contract is canonical — should `read`/`write` join
  `repo_root` (fixing the doc) or should the doc change (making the joining
  consumers' double-join-avoidance the rule)? `(needs human input)` — both
  readings are realized in the codebase; changing the impl is
  consumer-compatible for everyone who already joins (their paths stay
  absolute) and would break nobody found in-tree.
- Should `write_to` emit a placeholder (e.g. comment header) instead of `""`
  for all-default structs, or should `read_config_file` accept empty content
  as `Default`? `(partial: toml 0.8 round-trips an empty table as ""; the
  rejection is genesis's own choice in read_config_file)`

Evidence: [properties/config-file-round-trip-is-total.md](properties/config-file-round-trip-is-total.md)

---

### manifest-registry-converges-byte-deterministic — register/unregister is a convergent, byte-stable state machine

| | |
|---|---|
| **Type** | Safety |
| **Property** | Registering a set of tools in any order, then unregistering all of them, leaves no manifest behind; unregister on a missing manifest is a no-op; and two projects with the same logical tool set have **byte-identical** `tools.toml` regardless of registration order or process. |
| **Invariant** | `Always`: convergence (register/unregister leaves the documented end state: no manifest file, no `.genesis/` dir when empty); `Always`: byte-determinism of `write_manifest` for a given logical tool set. |
| **Check mode** | In-process with tempdirs; the byte-determinism half is **cross-process** in nature (HashMap iteration order), which no in-process single-run test can decide — see check-mode note. |
| **Seam** | `discovery::register`/`unregister` (`src/discovery.rs:154-215`), `Manifest.tools: HashMap<String, ToolEntry>` (`discovery.rs:52-56`), `write_manifest` → `toml::to_string_pretty` (`discovery.rs:186-193`). |
| **Generator sketch** | Generate a tool set (names from a fixed pool, detector type file/directory) and a registration order permutation; apply to a tempdir; snapshot bytes; apply the same set in a different order (second tempdir, or second process for full fidelity); compare. Then unregister all and assert absence. |
| **Predicted red/green** | Convergence **GREEN** (verified: two registers, two unregisters → no manifest, no `.genesis/`; ghost-unregister is a no-op). Byte-determinism **RED and flaky**: `Manifest.tools` is a `HashMap`, so `toml::to_string_pretty` emits tools in hash-map iteration order; the same logical state serialized with different insertion orders produced `[tools.dont]…[tools.wai]` vs. `[tools.wai]…[tools.dont]` across runs, and the in-process comparison **flips between passing and failing across test runs** (observed 3× fail / 2× pass in five runs) because per-process hash seeds differ. |
| **Why It Matters** | Every genesis-based tool writes this manifest during `init`; non-deterministic bytes make the file churn in diffs, defeat dedupe/caching, and make "did anything actually change?" unanswerable. |

**Open Questions:**

- Is byte-determinism itself required, or only semantic equality of the
  parsed manifest? The ticket states byte-deterministic; no doc in-tree
  pins it. `(needs human input)`
- Corrupt-manifest path: `read_manifest` returns `None` on a parse error and
  `register` then starts from an empty manifest, silently discarding every
  other tool's entry on the next write. Intentional fresh-start semantics or
  a data-loss hazard? `(partial: confirmed behavior in code; severity is a
  product call)`

Evidence: [properties/manifest-registry-converges-byte-deterministic.md](properties/manifest-registry-converges-byte-deterministic.md)
