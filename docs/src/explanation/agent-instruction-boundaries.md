# Agent Instruction Boundaries

**TL;DR:** `AGENTS.md` is a repository-owned, tool-neutral file. Each tool owns only its own delimited region in it, removes that region when it retires it, and is checked against what it publishes — a block or an artifact. `genesis::managed_block` provides the primitives; this page states the contract behind them.

`AGENTS.md` (agents.md) is plain Markdown with no required fields and no standard for managed regions. The fleet layers a private convention on top: a tool announces its region with a start marker, writes between it, and closes with an end marker. The convention is safe only while every participant agrees on who may write which bytes. The API surface is in [Modules Overview](../reference/modules.md).

## Ownership: one tool, one region

A managed block has exactly one owner: the tool that publishes it. That tool may write only the bytes between its own markers.

- `BlockInjector::inject` resolves the block by name in its `BlockRegistry`, finds that block's start and end markers, and splices the replacement over that span only. Bytes before the start marker, after the end marker, and every other tool's region are copied through unchanged.
- When the file exists but the block does not, injection prepends the tool's own block. It never rewrites an existing region to make room.
- Everything outside all markers — including the file's opening prose — belongs to the repository. `AGENTS.md` itself is never regenerated or deleted by an injector: tools own a region, not the file. The same rule applies per file when a region is also injected into `CLAUDE.md` or a tool's own config.

The failure this prevents is two writers on one region. While ddl's doctor demanded `<!-- OPENSPEC:START -->` after OpenSpec had retired it, the only available fixes were re-creating upstream's artifact or forking its text — either way two tools maintain one region. A checker may demand only what the publishing tool itself writes.

Registration mirrors ownership: each tool registers only the blocks it writes, and a registry holds at most one definition per block name. `has_block` and `read_block` let a checker inspect a region without writing it. A tool that encounters another tool's markers treats them as opaque bytes.

## Retirement: the publisher removes the region

A tool may stop publishing a region. Retirement has three parts:

1. **The publisher removes it.** Removal is a migration the tool performs or documents, not something another tool guesses on its behalf.
2. **No orchestrator writes it back.** `ddl init`, `wai init`, or any scaffolding cascade MUST NOT re-create markers for a tool that no longer publishes them. Re-creating a retired region freezes a dead layout.
3. **The checker verifies what the tool publishes now**, scoped to repositories that use that tool. A managed block is one possible signal; a published artifact path is another. For OpenSpec that artifact is `openspec/config.yaml`; for a repo that never used the tool there is no signal and no finding.

Scope is a predicate, not a membership list: a repo is checked for a tool when the tool's own directory exists (`.beads/`, `.wai/`, `openspec/`) or the repo pins the tool. Pins alone miss repos that consume a tool without installing it; universal membership reports on repos that never opted in.

Retirement also changes the documentation, not just the check: record where the content moved and point agents at the new location. Re-creating the region is not a migration.

A checker that follows the contract obeys four rules:

- The signal must be producible by the tool at the version the repo pins; otherwise the fix hint is a dead end.
- A retired marker may be accepted as a legacy advisory, never as a pass condition.
- A missing signal on a scoped repo is a warning, not a build failure; ddl keeps that exit-code contract.
- The fix hint names a command that can actually produce the signal.

## Marker dialects in use

`BlockDef::new(name)` generates `<!-- NAME:START -->` / `<!-- NAME:END -->`. A tool whose bytes differ declares them with `BlockDef::with_markers(name, start, end)` and the injector matches exactly those bytes. Markers are matched literally, never normalized.

| Marker pair | Form | Publisher |
| :--- | :--- | :--- |
| `<!-- DDL:START -->` / `<!-- DDL:END -->` | standard | `ddl init` |
| `<!-- WAI:START -->` / `<!-- WAI:END -->` | standard | `wai init` |
| `<!-- BEGIN BEADS INTEGRATION v:1 profile:minimal hash:… -->` / `<!-- END BEADS INTEGRATION -->` | non-standard: attributes on the start line, no `:START` suffix | `bd init` (AGENTS.md integration); `bd setup <recipe>` for tool-specific regions |
| `<!-- SPECODELIC:START -->` / `<!-- SPECODELIC:END -->` | standard | `spk init` |
| `<!-- ah:managed:start -->` / `<!-- ah:managed:end -->` | non-standard: lowercase | `ah init`, via `BlockDef::with_markers` |
| `<!-- DONT:START -->` / `<!-- DONT:END -->` | standard | `dont init`; repaired by `dont doctor --fix` |
| `<!-- TURU:START -->` / `<!-- TURU:END -->` | standard names; marker strings carry surrounding newlines | `turu sync` |
| `<!-- OPENSPEC:START -->` / `<!-- OPENSPEC:END -->` | **retired in 1.0.0** | OpenSpec before 1.0.0 |

WAI also registers `WAI:REFLECT:REF` with explicit markers; whisper's TURU block declares its markers with newlines so injection controls the blank lines around the region; bd's integration line carries `v`, `profile`, and `hash` attributes that no `BlockDef::new` name can express. That is why `with_markers` exists: the fleet's dialects are data, not special cases inside the injector.

## Drift: provenance footer plus content hash

Injection can append a provenance footer inside the end marker, after the content:

```text
<!-- provenance: generator=wai version=0.12.3 source=WAI sha=e91619f1 -->
```

`sha` is `content_sha8` over the injected content only — not the footer, not the markers. A checker recomputes the hash of the block body and compares it with the footer: a mismatch means the region was edited or generated from different content, detected without re-running the generator. A generator version bump does not move the hash for identical content, so a release is not mistaken for drift. `source` names the registry entry that owns the block, so a checker knows whose footer it is reading.

The footer is opt-in (`BlockInjector::with_provenance`); footer-less output is byte-identical to pre-footer releases and stays valid, just not self-checking. The same hash is used in the provenance footers of generated `llms.txt` and `llm.txt` artifacts, which is why `content_sha8` is public.

## Facts versus procedures

`AGENTS.md` carries stable facts: which tools are present, where their canonical instructions live, and the few commands that rarely change. Procedures — multi-step workflows, decision trees, command sequences — belong in Agent Skills (agentskills.io), whose `SKILL.md` carries a name and description in frontmatter and loads only when a task matches it.

That split is the legitimate reason for the OpenSpec retirement. A block that says "always open `openspec/AGENTS.md`" is a procedure pointer; once the procedure ships as a skill, the block is duplicate context in every prompt. A tool that moves procedural text to skills may stop writing a region, and the replacement is a recorded pointer, not a re-created block. Repository facts stay in the repository's own prose or in the tool's new artifact.

| Content | Home |
| :--- | :--- |
| Stable repository facts, tool inventory, canonical instruction paths | the repository's own `AGENTS.md` prose or the owning tool's block |
| Multi-step workflows, decision trees, command sequences | Agent Skills under `.agents/skills/<name>/SKILL.md` |
| Machine-checked project state | the tool's published artifact (for OpenSpec, `openspec/config.yaml`) |

The test is whether the content changes the agent's behavior on every turn or only when a task matches. Resident prose pays context cost on every prompt; a skill pays it on use. Procedure text that is still true belongs in a skill, not in a wider block.

## Worked example: OpenSpec 1.0.0

OpenSpec 1.0.0, "The OPSX Release" (PR #578), lists as a breaking change that tool-specific instruction files — `CLAUDE.md`, `.cursorrules`, `AGENTS.md`, `project.md` — are no longer generated. The workflow moved to agent skills (currently `.agents/skills/openspec-*`) and project facts moved to `openspec/config.yaml` (`context`, `rules`). The fleet's pin, 0.19.0, still writes the block; 1.0.0 stopped generating it.

Upstream's `openspec update` migrates a legacy repo. It removes authentic legacy marker blocks — fingerprint-gated in current releases, so a block whose text does not match a released template survives with hand-modified text intact — rewrites the surrounding file without deleting it, and deletes `openspec/AGENTS.md`, which is upstream-managed and obsolete. `openspec/project.md` is not deleted; the user migrates it into `config.yaml`.

ddl's `ddl.repo.blocks` check kept requiring the retired block and offered `ddl init` as the fix, but no installed OpenSpec could produce it. The `fix-agent-instrumentation-check` change is the correction: it replaces that entry with an artifact signal — `openspec/config.yaml` present means pass; `project.md` alone, or both files, yields a legacy or hybrid advisory naming the migration — scoped to repos with `openspec/` or a pin. ddl still writes its own block and never touches OpenSpec's retired one.

The fleet currently has three layouts: legacy block plus `openspec/AGENTS.md`, hybrid (`config.yaml` and `project.md`), and `project.md` only. The checker accepts all three with different signals; none of them justifies re-creating the block.

## Testing the boundary

The injector's region scoping is testable without running tools: seed `AGENTS.md` with two blocks, inject one, and diff the result — only the named block's lines may change. The evaluation harness describes exactly this scenario (managed-block boundary audits in [Creating Agent Evals](../how-to/evals.md)), and the contract's scenarios live in `openspec/specs/managed-block/spec.md`. A checker change follows the same rule: assert what the tool publishes, on the scope predicate, and assert that a retired region is never written back.