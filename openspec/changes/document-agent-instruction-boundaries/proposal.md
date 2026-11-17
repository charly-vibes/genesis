# Proposal: document-agent-instruction-boundaries

## Why

The fleet's `AGENTS.md` managed blocks are a private convention layered on an
open, tool-neutral file (agents.md: plain Markdown, no required fields, no
managed-region standard). Today the convention is documented nowhere except
code comments, and the fleet has just paid for that:

- `ddl doctor` required `<!-- OPENSPEC:START -->` in `AGENTS.md`;
- OpenSpec 1.0.0 ("The OPSX Release", PR #578) retired tool-specific
  instruction files, moving workflows to agent skills and project facts to
  `openspec/config.yaml`; 0.19.0 is the last release that writes a block;
- upstream's `openspec update` removes authentic legacy blocks, so ddl's fix
  hint could not produce the artifact it demanded;
- the fleet now has three layouts (nine legacy-block repos, one hybrid
  `project.md` + `config.yaml` repo, one `project.md`-only repo).

`genesis::managed_block` already provides the right primitives (`BlockDef`,
`BlockRegistry`, provenance footer, content hash, drift detection). What is
missing is the written contract that says who owns a region, what happens when
a tool retires a region, and what a checker may demand. Only a Rust doc
comment carries fragments of it, which neither maintainers nor agents find.

## What Changes

- New explanation page `docs/src/explanation/agent-instruction-boundaries.md`,
  linked from `docs/src/SUMMARY.md`:
  - the ownership rule — a tool owns only its own delimited region, and
    injection/update never touch another tool's markers;
  - the retirement rule — a tool that stops publishing a region removes it;
    no orchestrator re-creates a retired region, and a checker verifies what
    the tool publishes instead;
  - the marker dialects in use across the fleet (`<!-- NAME:START -->`,
    `<!-- BEGIN NAME -->`, lowercase variants) and how
    `BlockDef::with_markers` covers the non-standard ones;
  - the drift rule — provenance footer + content hash is how a stale block is
    detected without re-reading the generator;
  - the "facts vs procedures" split (AGENTS.md for stable facts, skills for
    procedures, per the Agent Skills standard) as the reason a tool may
    legitimately stop writing instruction text.
- `src/managed_block.rs` rustdoc keeps the API detail and points at the page
  instead of restating ecosystem rules.
- `docs/src/reference/modules.md` gains the pointer under `managed_block`.
- A spec requirement records the ownership and retirement contract so future
  changes can cite it.

## Impact

- Docs only plus one doc-comment edit; no API change, no behavior change, no
  new dependency.
- Affected spec: `managed-block` (new requirement).
- Non-goals: no `BlockDef`/`BlockRegistry` extension, no fleet-wide sweep, no
  change to how any tool renders its block.