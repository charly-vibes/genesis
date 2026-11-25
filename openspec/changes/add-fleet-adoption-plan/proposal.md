# Proposal: add-fleet-adoption-plan

Tracked as `genesis-svt` in beads (base_commit `3bffa47`).

Findings addressed (Rule-of-5 review of the fleet adoption review):
`DRAFT-001` (the fleet lane had no proposal and no tracked ticket).

Evidence: the fleet scan of the 18 Rust CLIs under
`para/areas/dev/gh/charly` — adoption matrix and per-tool module usage
(`genesis::` references counted per repo), plus the dependency pins read from
each repo's `Cargo.toml`.

Verification: self-reported validation for the adoption counts; the
`testaruda` row is marked `dep-only` (its source tree was unreadable during
the scan) and must be re-measured before it is used in any percentage.

Scope: the 18 Rust CLIs (`fotos` excluded); `testaruda` re-measured as its own row.

Depends on: nothing. Blocked by: nothing. Unblocks: the fleet-adoption lane
of `document-fleet-adoption-review`.

Capability path: none — this change tracks cross-repo work, not genesis
behavior (`skip_specs: true`).

Version bump: none (no genesis code changes).

Downstream repos: 12 — one ticket each, listed below.

## Why

The fleet-adoption review produced two lanes of work. The genesis-side lane
is covered by six proposals. The **fleet-side** lane — the work that lands in
the tools' own repositories — had no proposal and no ticket, so it existed
only as prose in a docs-page task. A review whose output is meant to seed
tickets must not lose half its findings to a missing home.

This change is that home: one ticket per repo, with the scope, the blast
radius, and what blocks it. Nothing here changes genesis; everything here
*points at* a change in another repository.

## What Changes

No genesis code changes. The change adds the per-repo ticket set:

| Repo | Scope | Blast radius | Blocked by |
|---|---|---|---|
| kaya | adopt `genesis::envelope` + `genesis::cli`; its own `envelope.rs` becomes a thin wrapper | output shape unchanged if the wrapper keeps the fields; consumers read `data` | none |
| sunshine | adopt `genesis::envelope`; map its envelope kinds (`Render`, `Audit`) onto genesis kinds | envelope kind values change — a consumer matching on `render`/`audit` must be updated | none |
| funkalization | adopt `genesis::envelope` + `genesis::cli`; its own `Error` type stays | error text moves from `code: message` to the envelope's error shape | none |
| wild | keep the `v1` envelope as `data` inside a genesis envelope | additive: the v1 payload is unchanged, a genesis shell wraps it | none |
| bajan | bump `genesis-vibes` 0.8 → 0.13.x | **breaking** — `Envelope::success` gained arguments; call sites in `bajan/src/cli.rs` must be rewritten | none |
| evallerina | move the frozen git tag `v0.12.3` → a 0.13.x release | breaking for any 0.12-only API it still calls | `fix-envelope-hint-channel` (it reads `parse_envelope`) |
| wai | migrate `wai/src/commands/way/hooks.rs` + `guide/hooks.rs` onto `genesis::git_hooks` | hook scope semantics (`--local`-only) must be preserved | none — beads `genesis-orq` |
| dulce-de-leche | migrate `dulce-de-leche/src/gates/lefthook.rs` onto `genesis::git_hooks` | gate wiring changes; ddL's own `with_sharing_retry`-style platform handling stays | none |
| pretender | migrate `pretender/pretender/src/cli/hooks.rs` onto `genesis::git_hooks` | hook scope semantics preserved | none — beads `genesis-q4k` |
| 9 tools without feedback | adopt `genesis::feedback` (kaya, evallerina, sunshine, wild, bajan, sad, funkalization, podras, testeruda) | additive: a new `feedback` subcommand | none — beads `genesis-dvf` covers testeruda |
| testeruda | **re-measure** adoption (source tree unreadable during the scan) then adopt | unknown until measured | none |
| 4 tools with partial adoption | fill the missing modules: whisper (envelope, cli, config, status, aix), podras (doctor, config, feedback, guide, status), sad (guide, config, doctor, feedback, cli), gently (envelope, cli, config, feedback, status) | additive per module | none |

## Impact

- **Affected code**: none in genesis. Every row above is a change in another
  repository.
- **Affected specs**: none — `skip_specs: true`, because no genesis behavior
  changes.
- **Beads**: one issue per row, filed at approval in the *owning* repo's
  tracker; this change holds the index.
