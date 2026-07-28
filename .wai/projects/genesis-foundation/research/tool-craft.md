# charly-vibes Tool-Craft Playbook

> Status: **Draft** — normative guidance for CLI tools in the charly-vibes
> suite, mined from wai and generalized. Tracked by `wai-bdqw.*`.
>
> **How to read this doc.** Two kinds of rules appear below:
> - ✅ **Met today** — every shipped tool already does this. Treat as law.
> - 🎯 **Target** — the suite does *not* yet meet this; it is the direction.
>   Each 🎯 rule cites the closing ticket. Do not file tickets against a
>   tool for a 🎯 rule unless the ticket says so.
>
> Every claim is backed by a concrete artifact; when a new tool deviates, it
> does so deliberately and documents why. This is the suite-level source the
> `wai-bdqw.6` standardization work traces to.

Audience: new tools (crua, livin, vampiro) and shipped ones (pretender, dont,
espectacular, testaruda).

---

## 1. Design principles (the four wai established)

wai's README and `docs/src/introduction.md` codify four principles. They are
not slogans — each has a concrete mechanism. Adopt all four.

| Principle | wai mechanism (evidence) | Rule | Status |
|---|---|---|---|
| **Desire Path Alignment** | `wai status` computes a `Pattern` (e.g. `ReadyToImplement`) and prints a `command:` the user can copy-paste. See `docs/src/advanced/workflow-detection.md`. | The default command of every tool *suggests the next command* based on state, not just report state. | 🎯 (wai only; port via `wai-bdqw` subtree) |
| **Self-Healing Errors** | `Suggestion` enum (`src/suggestions.rs`): `DidYouMean`, `WrongOrder`, `ContextHint`, `Fix`. `doctor::CheckResult` carries a `fix: Option<String>` *and* a `fix_fn` that can apply it. | Every error path emits a fix or a "Run: …" footer. Never a bare "error: X". | 🎯 (only wai has `Suggestion`; `doctor` in 4/5) |
| **Progressive Disclosure** | `-v` / `-vv` / `-vvv` reveal tiers; `help::HelpContent` splits `options` vs `advanced_options` vs `internals`; `init` asks only for a project name. | Simple by default, powerful when asked. Three verbosity tiers, one JSON escape hatch. | 🎯 (wai pattern; adopt suite-wide) |
| **Context-Aware** | Plugins auto-detected by workspace markers (`.beads/`, `openspec/`); suggestions adapt. See `workflow-detection.md`. | Detect, don't configure. A tool in a repo with `.wai/` should know wai is present. | 🎯 (reciprocal awareness not yet built; `wai-bdqw.1`/`.2`–`.5`) |

A fifth principle the suite adds:

| **Agent Experience (AIX)** | `llms.txt` + `llm.txt`, `AGENTS.md` managed blocks, `prime`/`close` session loop, ubiquitous language, `--json` everywhere. | A human and an agent must both be able to drive the tool from cold. See §5. | 🎯 (partial; see matrix) |

---

## 2. CLI verb grammar

wai's verb taxonomy (from `src/cli.rs` and `docs/src/commands.md`) is
deliberate. But wai is an *orchestrator* — its surface is larger than a
domain CLI needs. Split the verbs accordingly.

### 2.1 True suite minimum (every tool ships these) ✅

| Verb | Purpose | wai evidence | Suite status |
|---|---|---|---|
| `init` | Create the tool's on-disk state in the current repo | `commands/init.rs` | ✅ all 5 |
| `doctor` | Health check with `--fix` that applies fixes | `commands/doctor/` | ✅ all 5 |
| `config` | Inspect/edit tool config | `commands/config_cmd.rs` | ✅ all 5 |
| `completions` | Shell completions | `cli.rs` `Commands::Completions` | ✅ all 5 |

These four are the only verbs mandated across the whole suite. Domain CLIs
(dont, espectacular, testaruda) are *not* required to ship `status`/`ls`/
`search`/`prime`/`show` — those are orchestrator concerns (see 2.2).

### 2.2 wai-specific workflow verbs (not mandated for domain CLIs) 🎯

`status` (desire-path report), `prime`/`close` (session loop), `show`/`ls`/
`search` (entity navigation). wai ships these because it is the suite's
orchestrator. A domain CLI that needs any of them may adopt it, but a
smaller domain surface is correct, not a defect. Current adoption:

| Verb | wai | pretender | dont | espectacular | testaruda |
|---|---|---|---|---|---|
| `status` | ✓ | – | – | – | – |
| `prime` | ✓ | – | ✓ | – | – |
| `show` | ✓ | – | ✓ | – | – |
| `ls` | ✓ | – | – | – | – |
| `search` | ✓ | – | – | – | – |

### 2.3 Domain verbs that follow a convention ✅

- **Noun-first creation**: `wai new project`, `wai add research`, `wai add
  design`, `dont define`, `dont conclude`. The verb (`new`/`add`/`define`) is
  generic; the noun disambiguates. Reuse `new` (top-level entity) and `add`
  (append to an entity) rather than minting `create-project`/`mkresearch`.
- **Lifecycle verbs as a closed set**: `new → move → close`. `move` is renamed
  via `#[command(name = "move")]` because `mv` is a shell keyword. `close`
  ends a session/entity and emits a handoff.
- **Phase/lifecycle queries**: `phase` (subcommand) with `next`/`set`/`show`.
  wai-only; not mandated.

### 2.4 Reflective verbs (wai-specific, not a suite convention) 🎯

`way` (best-practices audit of the host repo), `why` (LLM oracle), `reflect`
(synthesize context into AGENTS.md). These depend on wai's LLM features,
which no other suite tool has. **Do not mandate `way`/`why`/`reflect`
suite-wide.** Other tools may add their own reflective verbs with a
one-line rationale in their `docs/src/commands.md`.

### 2.5 Reserved verbs (do not reuse) ✅

Before naming a new verb, check Appendix A.2. Currently reserved with a
non-obvious meaning: `report` (pretender, espectacular — *render a local
check output*, not file an issue). The feedback-filing feature uses
`feedback` precisely to avoid this collision.

---

## 3. Flag consistency

Inconsistencies today (the L-tier findings) exist because this was never
written down. Pin it here. 🎯 — `wai doctor --suite` (`wai-bdqw.8`) will lint.

| Concern | Rule | wai evidence |
|---|---|---|
| Verbosity | `-v` (sections), `-vv` (internals), `-vvv` (trace). Exactly three tiers. | `help.rs` tier split |
| Machine output | `--json` prints a stable envelope to stdout; human diagnostics go to stderr. | `output::print_json`, `json.rs` |
| Quiet | `-q` / `--quiet` suppresses suggestions, not errors. | `workflow-detection.md` |
| Color | Respect `NO_COLOR`; never hardcode ANSI. | `owo-colors` in deps |
| Value names | `--flag <VALUE_NAME>` uppercase, e.g. `--file <FILE>`, `--base <REF>`. | `cli.rs` `value_name` attrs |
| Env passthrough | Flags that read env use `clap`'s `env` feature; document the var in `HelpContent.env_vars`. | `cli.rs` `#[arg(env_name)]` |
| Confirmation | mutating/remote ops default to interactive confirm; `--yes` skips for AFK use. | suite convention (wai sync `--yes`, dont `check`) |
| `--from-*` provenance | any flag that pulls state from an external source names its source (`--from-last-error`, `--base <REF>`). | testaruda `--base`/`--head` |

---

## 4. Self-healing errors and next-step guidance

wai's `Suggestion` enum is the canonical shape. 🎯 **Target**: every tool
reproduces it (only wai does today — see Appendix A.3).

```rust
enum Suggestion {
    DidYouMean { original, suggestion },   // typo — Jaro/Levenshtein
    WrongOrder { original, correct },      // "project new" -> "new project"
    ContextHint { message, path },         // "run from project root: <path>"
    Fix { description, command: Option },  // the workhorse: a one-liner to run
}
```

### Rules

1. **Every error has a footer.** Either a `Suggestion::Fix` ("→ Run: …") or,
   if no fix is known, a `Suggestion::ContextHint` pointing at docs/the
   `doctor` check most likely to help. A bare `Error: X` is a bug. 🎯
2. **Doctor checks carry their own fix.** `CheckResult { status, message, fix,
   fix_fn }` — the `fix` string is what `doctor` prints; `fix_fn` is what
   `doctor --fix` runs. Never a check that only *describes* drift. ✅ (the 4
   tools that ship `doctor` all follow this)
3. **Typos are free-text.** Unknown subcommands run `suggest_typo` against the
   valid command list (threshold ~0.6). wai also catches argument-order
   inversions (`WrongOrder`). 🎯
4. **Status suggests, it doesn't gate.** Suggestions are guidance; `--quiet`
   hides them. A tool that nags is a tool that gets aliased away.
5. **Errors link to the feedback loop.** When a self-healing fix is not
   available, the footer offers `feedback` (see `agent-issue-reporting.md`) so
   friction flows upstream. 🎯

---

## 5. Agent experience (AIX)

A tool is AIX-grade if a cold-started agent can use it correctly without
reading prose. wai's pattern:

### 5.1 The three AIX artifacts (per repo)

| File | Audience | Layer | wai evidence |
|---|---|---|---|
| `llms.txt` | LLM link-crawlers | dense summary + links | `llms.txt` |
| `llm.txt` | LLM inline context | flat command cheatsheet | `llm.txt` |
| `AGENTS.md` | in-session agent | managed blocks injected by `wai sync` | `managed_block.rs` |

`AGENTS.md` is **generated, not hand-written**, from managed blocks:
`<!-- WAI:START -->`, `<!-- OPENSPEC:START -->`, `<!-- DONT:START -->`,
`# ah:managed:start`. The block is *slim* (Layer 1: orient + point at
`.wai/AGENTS.md` for the full reference) — progressive disclosure applied to
agent instructions. 🎯 **Target**: every tool that participates in a
managed-block system ships the injector (wai's `managed_block.rs`) and the
block. Today only pretender still lacks the injector; testaruda now has it
via genesis adoption (closes `wai-bdqw.7` for testaruda). testaruda also now
ships `llm.txt`.

### 5.2 The session loop 🎯

`prime` (start) → work, capturing decisions → `close` (handoff). `prime` is
*graceful on empty state*: if no projects exist, it prints a helpful prompt
instead of crashing (`commands/prime.rs`). 🎯 **Target**: every tool with
session state ships `prime`/`close` and degrades gracefully when empty.
Today: wai and dont have `prime`; espectacular/testaruda/pretender do not.

### 5.3 Ubiquitous language 🎯

wai stores domain terms under `.wai/resources/ubiquitous-language/` with a
`README.md` navigation index and per-context files. The managed block tells
agents to read the index first, then only the relevant context file. 🎯
**Target**: any tool with domain jargon (crua: cost pattern/hotness/tier;
livin: shape/boundary value/reachability; vampiro: seam/CIR/axis) ships a
ubiquitous-language index and names it in its AGENTS.md block.

### 5.4 JSON everywhere ✅ (target across all commands)

Every command that produces structured output accepts `--json` and emits a
**stable, versioned envelope**. wai's `PrimePayload` and `status --json` are
the model. Agents compose tools by piping JSON, not by scraping prose.

---

## 6. Brownfield integration

`docs/src/how-to/adopt-wai.md` is the template. The non-obvious rules:

1. **`init` is non-destructive.** It creates `.wai/` and stops. It does not
   reformat, move, or infer phases from existing code. ✅
2. **Import, don't backfill.** `wai import .cursorrules` and
   `wai add research --file docs/notes.md` let a brownfield repo contribute
   what it already has. The doc explicitly says: *"Don't try to backfill all
   history. Capture decisions going forward. The value compounds."* ✅
3. **Set the phase to reality.** `wai phase set implement` lets a repo that's
   already shipping acknowledge where it is instead of pretending to start at
   research.
4. **Doctor before trusting.** `wai doctor` + `wai way` verify the workspace
   and the host repo's practices before the tool is relied on.
5. **Commit the state — but split text vs DB.**
   - **Text state** (`.wai/`, `.espectacular/` — toml/md): committed
     alongside code. Tool state is repo state.
   - **DB-backed state** (`.dont/` SQLite/cozo, `.beads/`): **gitignored**,
     with a serialised text mirror committed if cross-machine reproducibility
     matters (e.g. `dont export` to a checked-in dump). Committing binary DBs
     causes merge conflicts and bloats history.
   The earlier blanket rule "commit `.dont/` too" was wrong — it lumped a
   SQLite store with text artifacts.

🎯 **Target**: every tool ships an `adopt-<tool>.md` following this shape, and
`init` is always non-destructive with an `import` path for pre-existing
artifacts.

---

## 7. Cross-tool integration conventions

These make the suite compose (and are what tickets `wai-bdqw.2`–`.5` detect):

| Signal | Owned by | Detected by | Status |
|---|---|---|---|
| `.wai/` | wai | every tool's `status` | 🎯 (`wai-bdqw.2`–`.5`) |
| `.beads/` | beads (`bd`) | wai plugin system | ✅ |
| `openspec/` | openspec | wai plugin system | ✅ |
| `.dont/` | dont | wai (planned, `wai-bdqw.4`) | 🎯 |
| `.espectacular/` | espectacular | wai (planned, `wai-bdqw.3`) | 🎯 |
| `testaruda.toml` | testaruda | wai (planned, `wai-bdqw.2`) | 🎯 |
| `pretender.toml` | pretender | wai (planned, `wai-bdqw.5`) | 🎯 |
| `*-ears-spec.md` | crua/livin/vampiro | wai (planned, `wai-bdqw.9`) | 🎯 |

🎯 **Target**: a tool participates by (a) owning one signal directory/file,
(b) carrying the managed block for tools it depends on, (c) exposing state
via `--json`. No tool reads another's internal store; it reads the other's
`--json` output or its public signal files.

---

## 8. What to copy verbatim from wai when bootstrapping a new tool

- `justfile` shape: `default`, `build`, `test`, `lint`, `fmt`, `fmt-check`,
  `ci`, `validate` (= `ah check`), `prime`, `status`. (See `wai-bdqw.6`.)
- `lefthook.yml` with `# ah:managed:start` pre-commit + `testaruda select
  --safe` pre-push.
- `_typos.toml`, `book.toml`, `llms.txt`/`llm.txt`, `AGENTS.md` managed blocks.
- `Cargo.toml` metadata shape. **Recommendation** (pending `wai-bdqw.6`
  ratification): `edition = "2024"`, `license = "Apache-2.0"`, SemVer `0.x`
  for new tools (wai's CalVer is grandfathered — do not extend CalVer to new
  tools without an explicit decision).
- The `Suggestion` enum and `doctor::CheckResult` shapes from §4 (🎯 until
  ported; the port is a candidate for the `dont-2j6o` shared crate).

---

## Appendix A: Suite compliance matrix

Derived from the extended investigation (2026-07-27): command-enum grep +
AIX module audit across `wai`, `pretender`, `dont`, `espectacular`,
`testaruda`. The trio (crua/livin/vampiro) have no `src` yet.

### A.1 True-suite-minimum verbs

| Verb | wai | pretender | dont | espectacular | testaruda |
|---|---|---|---|---|---|
| `init` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `doctor` | ✓ | ✓ | ✓ | ✓ | ✗ |
| `config` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `completions` | ✓ | ✓ | ✓ | ✓ | ✓ |

### A.2 Reserved / taken verbs (collision check)

| Verb | wai | pretender | dont | espectacular | testaruda | meaning |
|---|---|---|---|---|---|---|
| `report` | – | ✓ | – | ✓ | – | **render local check output** (human/md/html/matrix) — *not* "file an issue" |
| `feedback` | – | – | – | – | – | **free** — used by the agent-issue-reporting feature |

### A.3 AIX & self-healing machinery

| Artifact | wai | pretender | dont | espectacular | testaruda |
|---|---|---|---|---|---|
| `llms.txt` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `llm.txt` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `AGENTS.md` | ✓ | ✓ | ✓ | ✓ | ✓ |
| `managed_block.rs` (injector) | ✓ | ✗ | ✓ | ✓ | ✓ |
| `Suggestion` enum (self-heal) | ✓ | ✗ | ✗ | ✗ | ✓ |
| `doctor` command | ✓ | ✓ | ✓ | ✓ | ✓ |

**Reading the matrix.** A ✓ in every column = ✅ suite rule (law). Any row
with a ✗ = 🎯 target; the gap is a ticket, not a defect in the tools that
diverge. The `Suggestion` enum row is the clearest case: only wai has it, so
"every tool reproduces it" is a target, not a current rule.

### A.4 Provenance (git history)

The self-healing/managed-block/doctor machinery was deliberately built in
wai and never ported: `feat(suggestions): add self-healing error foundation
with strsim`, `feat: rewrite managed block with behavioral guidance for
LLMs`, `Inject wai managed block into AGENTS.md on init, check in doctor`.
No other repo's history references these patterns. Confirms "we worked hard
in wai mainly to figure those" — and that the port is real work, not a
copy-paste.

---

## Traceability

This playbook is the source for: `wai-bdqw.1` (toolchain doc), `wai-bdqw.6`
(suite standardization), `wai-bdqw.8` (config linter), and the
`agent-issue-reporting` feature. When a rule here changes, update those
tickets' acceptance criteria in the same change. New gaps surfaced by the
compliance matrix (e.g. porting `Suggestion` to the other four tools) need
their own tickets filed against `wai-bdqw` or the respective tool repo.
