## Module layout

```
genesis/
  src/
    lib.rs          # re-exports
    envelope.rs     # from dont/src/envelope.rs (dont-2j6o)
    suggestions.rs # from wai/src/suggestions.rs
    managed_block.rs
    aix.rs
    feedback.rs     # redactor, context bundle, error scratch
    suite_linter.rs # the wai doctor --suite checks
  openspec/
  Cargo.toml
```

## Boundary rule (enforced in review)

A module is accepted into genesis **only if** two or more suite tools need
it. The reviewer checklist for any new module:

1. Do at least two tools need this? (If one → reject, keep in the tool.)
2. Is it domain logic? (If yes → reject, never extract.)
3. Does it have a stable, minimal interface? (If not → keep in donor until stable.)

## Distribution

Git dependency, no crates.io until stable (per `dont-2j6o`):

```toml
[dependencies]
genesis = { git = "https://github.com/charly-vibes/genesis", tag = "v0.1.0" }
```

Pin by tag, not branch, so dependents are reproducible.

## Module layout note

`genesis/openspec/` already exists (this change plus `project.md`); the
`openspec/` entry in the layout above is retained for clarity but is **not**
to be created or overwritten by task 1.1.

## Versioning

SemVer. Breaking changes:

1. Bump genesis's minor version and tag.
2. Open a coordinated change updating every dependent's tag pin.
3. Deprecate the old surface for one release cycle before removal.

## Relationship to dont-2j6o

This change **supersedes** `dont-2j6o`. The envelope extraction is task 2.1
here. dont-2j6o's beads epic should be closed as superseded once task 2.1
lands, with its design notes (concrete `Envelope` struct, git-dep decision)
carried into this change's `envelope` module verbatim.

## What is NOT in genesis

- Domain metrics, stores, engines, scenario contracts, Datalog, PARA state,
  static analysis (call/module/effect/trust-boundary checks).
- Tool-specific command verbs beyond the true suite minimum
  (`init`/`doctor`/`config`/`completions` wiring helpers are shared; the
  domain verbs stay in each tool).
- Any LLM oracle / `why` / `reflect` logic (wai-specific).
