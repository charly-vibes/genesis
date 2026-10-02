# Change: Fix lefthook managed-block wiring placement (nest inside `commands:`)

## Why

`lefthook::ensure_wired()` inserts the managed block **directly after the stage
key** (`pre-commit:`), per the deployed spec. With the canonical lefthook
config shape — every stage carries its commands under a `commands:` mapping —
this produces a stage-level key that **lefthook silently ignores at runtime**
(only `lefthook validate` rejects it). Downstream wiring (espectacular's
`ah check`, GH#31) is a silent no-op: hooks run, gate never fires.

Verified downstream on ah 0.9.0 + lefthook 1.13.6: `git commit` ran the
existing `trailing-whitespace` command and never `ah check`. The current
test helper content (`  commands:\n    ah:...`) compounds the flaw: if the
stage already has `commands:`, the injected mapping is a duplicate YAML key.

Tracked as genesis-r99. Root-cause context in
charly-vibes/espectacular#31.

## What Changes

### `ensure_wired` nests the block inside the stage's `commands:` mapping

- Anchor resolution unchanged (column-0 stage key, comment lines skipped,
  quoted keys unanchorable).
- **NEW**: within the stage's section, locate the `commands:` key line and
  insert the block after it (content lands as children of `commands:`).
- **NEW**: if the stage section has no `commands:` key, emit
  `  commands:` after the stage anchor line, then the block.
- **NEW**: appended missing stages also get `  commands:` + block.
- **Caller content contract change**: content is now the *entries inside*
  `commands:` (e.g. `"    ah:\n      run: ah check\n"`, 4-space indent),
  not a stage-level mapping. Content is inserted verbatim — no
  re-indentation, so multi-line values stay intact.
- Idempotence and missing-config behavior unchanged; `is_wired()`
  (stage-section scan) unaffected — nested blocks are still inside the
  stage section.

## Impact

- **Affected specs**: `git-hooks` — MODIFIED `Lefthook managed-block wiring`
  (placement scenarios rewritten)
- **Affected code**: `src/git_hooks.rs` (`ensure_wired` insertion logic +
  wiring tests; `ah_content()` helper becomes inner-mapping form)
- **Downstream**: espectacular pins the new placement and flips its
  wrongly-pinned `init_cli.rs` assertion after the pin bump
  (espectacular-391); espectacular's `LEFTHOOK_AH_COMMAND` becomes
  4-space-indented inner content.
