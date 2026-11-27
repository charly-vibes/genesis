# managed-block-markers-stay-unique-and-idempotent — evidence

Extends the suite-wide entry (`charly/antithesis/scratchbook/properties/genesis.md`,
2026-11-16, commit `a6a5150`); reflects genesis `ddf412d`.

## What led to this property

The 2026-11-16 scan found `inject_into_existing` locating START and END
independently, splicing END-before-START files into duplicated tails. Since
then a guard was added: `splice::block_span` (`src/managed_block/splice.rs:22-37`)
searches the end marker only **after** the start marker, and a regression
test `test_inject_end_marker_before_start_is_not_corrupted`
(`src/managed_block/tests/splice.rs:71`) pins the behavior.

## Search-for-the-end-after-the-start fixes the old bug and creates two new ones

The guard makes `block_span` return `None` whenever no END follows the first
START occurrence, and makes the span start at the **first** START occurrence
even if it is prose quoting the marker. Tracing both branches:

1. **Quoted/stray START, no END after it** → `block_span` → `None` →
   `inject_into_existing` takes the `prepend_block` branch (`splice.rs:9-15`)
   → the file now holds the prepended block **plus** the original START
   occurrence: two START markers, one END. `has_block` still reports `true`.
2. **Quoted START before the real block** → `block_span` returns the span
   from the quote to the real block's END → `splice_block` replaces
   everything in between: user text between the quoted marker and the real
   block is destroyed.
3. **Stray END before the real block** (the old scan's case, inverted) → the
   splice is correct but the stray END survives: the file ends with two END
   markers. The existing regression test explicitly asserts
   `matches("<!-- WAI:END -->").count() == 2` here — this outcome is
   **codified as intended**, so it is a semantics/policy question, not a bug
   claim. The ticket's phrasing "exactly one ordered marker pair" is
   therefore stronger than the codebase's current contract; the catalog
   property is written with "at most one … any START precedes any END"
   relaxed accordingly and flags the policy decision as an open question.

## Execution record

Plain-cargo scratch tests (`antithesis/scratch-hegel/tests/predictions.rs`):

| Test | Outcome |
|---|---|
| `p2_inject_idempotent_on_clean_file` (clean file: one ordered pair; re-inject byte-identical) | GREEN |
| `p2_end_before_start_file` (stray END survives → 2 END markers) | RED on the "exactly one pair" reading; matches the existing test's pinned semantics |
| `p2_quoted_start_marker_no_end` (quoted START, no END → 2 START markers) | RED |
| `p2_quoted_start_before_real_block` ("# Real section / precious user notes" spliced away) | RED — data loss |

Hegel-driven run (`tests/hegel_managed_block.rs`, `#[hegel::test]`,
`test_cases = 200`, `static-engine` feature, rustc 1.100.0):

- Found a failure and **shrank it to the minimal case** `picks = [2]` — a
  file consisting solely of a START marker (mode 1) — and emitted a working
  `#[hegel::reproduce_failure("AAMAAAABAQAKAQAAAAIBAA==")]` blob.
- Total wall time for the failure: one `cargo test` invocation; genesis +
  scratch crate compiled in 8.7 s with `static-engine` (no nested engine
  build, no runtime `.so`).
- This validates the "ideal Hegel fit" claim of the prior scan: the bug is
  reachable from short structured text and the engine's string-list generator
  finds it without a custom strategy.

## Check mode, seam, generator

- Check mode: in-process, tempdir per case.
- Seam: `BlockInjector::inject` (`src/managed_block.rs:249-270`); no
  SUT-side instrumentation needed — the property is workload-visible because
  the file content is the observable state.
- Generator: pool-index list (≤8 lines: prose / heading / raw START / raw
  END / prose-quoting-START / blank) mapped to a file body; fixed block
  content; assertions on marker counts, ordering, non-marker survival, and
  second-inject byte identity.

## Consumer version skew

Managed blocks are used by tools that rewrite `AGENTS.md`-class files:

- **Reaches on rebuild** (`^0.14`, locked 0.14.0): `dont`, `wai`,
  `testaruda`, `evallerina`, `dulce-de-leche`.
- **Does not reach** without bumping: `whisper` (`^0.13.1`, locked 0.13.1),
  `gently-cli` (`^0.13.2`), `specodelic` (locked 0.13.2),
  `specodelic-gre-wt` (locked 0.11.1), `bajan` (locked 0.8.3).

The module header records it was "Ported from `wai/src/managed_block.rs`";
wai also carries its own copy for older pins, so fixes can lag per tool even
on the same genesis version. A fix here should be checked for the same
defect in wai's forked copy.

## Open Questions

- Prepended-vs-adopted semantics for a start-without-end file `(needs human
  input)` — see catalog.
- Should stray-END files be normalized to one pair? `(needs human input)` —
  pinned as intended by `test_inject_end_marker_before_start_is_not_corrupted`.

## Investigation Log

#### Does the 2026-11-16 END-before-START defect still reproduce?

- Examined: `src/managed_block/splice.rs` (current `block_span` guard),
  `src/managed_block/tests/splice.rs:71` (regression test added since).
- Found: the independent-START/END-search defect is fixed and tested; the
  fix's own test accepts a surviving stray END (2 END markers).
- Not found: any handling for start-without-end beyond falling back to
  prepend, which duplicates the marker.
- Conclusion: property restated; new RED modes documented above.
