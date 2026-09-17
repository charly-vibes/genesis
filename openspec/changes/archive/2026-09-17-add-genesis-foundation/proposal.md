# Change: Add genesis foundation

## Why

The suite consistency evaluation (2026-07-27) found that four-of-five CLI
tools reimplement the same cross-cutting infrastructure independently, and
most do it incompletely:

- Only **wai** has the `Suggestion` self-healing enum; dont/espectacular/
  pretender/testaruda have none.
- Only **wai/dont/espectacular** ship a managed-block injector; pretender and
  testaruda have none.
- Four tools emit **incompatible JSON envelopes** for the same "give me
  status/health" family of commands (`dont-2j6o` F1).
- Every tool reimplements `-v/-vv/-vvv`, `--json`, `NO_COLOR`, value-name
  conventions, and `--yes` from scratch — inconsistently (the L-tier findings).
- The `feedback` feature and the `wai doctor --suite` linter are not built
  anywhere yet.

`dont-2j6o` already proposed extracting the JSON envelope into a shared
crate. This change **generalizes** that effort: one shared crate,
`genesis`, owns *all* cross-cutting CLI/AIX/self-healing infrastructure,
and each tool depends on it instead of reimplementing or copying.

## What Changes

- New crate `genesis` at `github.com/charly-vibes/genesis`, distributed
  as a git dependency (no crates.io publish until the interface stabilizes).
- Six modules, each with a named donor where the canonical implementation
  already exists:
  - `envelope` — the shared JSON envelope. **Supersedes `dont-2j6o`**, which
    becomes the extraction task for this module (dont is the donor).
  - `suggestions` — `Suggestion` enum (`DidYouMean`/`WrongOrder`/`ContextHint`/`Fix`) + `suggest_typo` engine (wai is the donor).
  - `managed_block` — the `<!-- …:START -->` injector + block format (wai/dont/espectacular donors).
  - `aix` — `llms.txt` / `llm.txt` / `AGENTS.md` block generation.
  - `feedback` — redactor + context-bundle serializer + error scratch (new; see the agent-issue-reporting playbook in genesis `.wai` research at `.wai/projects/genesis-foundation/research/agent-issue-reporting.md`).
  - `suite_linter` — the `wai doctor --suite` checks (`wai-bdqw.8`): testaruda.toml schema, pretender.toml presence, ah/dont gate wiring, AGENTS badge/block match.
- **Boundary rule (normative):** a capability belongs in genesis only if
  two or more suite tools need it. Domain logic is never extracted.
- Per-repo adoption proposals (separate changes, one per tool repo) depend
  on this one and are blocked until genesis's interface is tagged stable.

## Impact

- Affected specs: new `genesis` capability (this change).
- Affected code: new crate; no tool code changes in this change (those are
  the per-repo adoption proposals).
- **Supersedes** `dont-2j6o` (beads epic) — same idea, broader scope; the
  envelope extraction becomes task 2.1 here.
- Relates to `wai-bdqw.8` (suite linter) and the `feedback` feature
(genesis `.wai` research, `.wai/projects/genesis-foundation/research/agent-issue-reporting.md`).
