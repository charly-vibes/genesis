# Modules Overview

**TL;DR:** Reference table of all genesis-vibes modules with key types, traits, and entry points.

## Module Map

| Module | Status | Key Types / Traits | Entry Point |
| :--- | :--- | :--- | :--- |
| `envelope` | stable | `Envelope<T>`, `EnvelopeKind`, `ErrorResult`, `ReceiptMeta`, `TerminalOutcome`, `set_author()` | `Envelope::success()`, `Envelope::error()`, `Envelope::with_receipt()` |
| `guide` | stable | `Verbosity`, `Output`, `CliVerbosity`, `CliFormat`, `OutputFormat`, `ErrorSink`, `GuideBuilder`, `Guide` | `Output::success()`, `Output::emit()` |
| `suggestions` | stable | `Suggestion`, `SuggestionEngine`, `CommandRegistry` | `SuggestionEngine::new()` |
| `managed_block` | stable | `BlockDef`, `BlockInjector`, `BlockRegistry` | `BlockInjector::new()` |
| `aix` | stable | `ProjectMeta`, `ModuleEntry`, `LlmSection`, `TokenCost` | `generate_llms_txt()`, `generate_llm_txt_bounded()`, `estimate_token_cost()` |
| `config` | stable | `ConfigFile` trait, `ConfigRegistry`, `ConfigStore` | `ConfigFile::read()` |
| `fixture` | stable | `Fixture`, `FixtureError` | `Fixture::new()` |
| `feedback` | stable | `handle_feedback()`, `FeedbackArgs` | `handle_feedback()` |
| `suite_linter` | stable | `LintCheck` trait, `LinterRegistry`, `LintResult`, `Severity` | `LintCheck::check()` |
| `doctor` | new | `DoctorCheck` trait, `DoctorRunner`, `DoctorReport`, `CheckStatus` | `DoctorRunner::new()` |
| `cli` | new | `generate_completions()`, `maybe_print_version_json()` | `generate_completions()` |
| `status` | new | `StatusContributor` trait, `StatusBuilder`, `StatusLevel`, `StatusSection` | `StatusBuilder::new()` |
| `scaffold` | new | `Scaffold`, `ScaffoldResult` | `Scaffold::new()` |
| `discovery` | new | `scan()`, `register()`, `unregister()`, `Manifest`, `DetectedTool` | `scan()`, `register()` |

---

## envelope

**Signature:** `genesis::envelope`

Structured CLI output envelope. Every command returns an `Envelope<T>`.

### Key Types

| Type | Description |
| :--- | :--- |
| `Envelope<T>` | Generic output envelope with `ok`, `data`, `error`, `warnings`, `hints`, `meta`, optional `receipt` |
| `EnvelopeKind` | Closed enum: `Ok`, `Error`, `Empty`, `List`, `Check`, `Doctor`, `Version`, `Stats`, `Info`, `Warning` |
| `ErrorResult` | Error with mandatory `remediation` field (constructor returns `Err` if empty) |
| `Meta` | Observability metadata: `duration`, `transaction_id`, `request_id`, `author` |
| `Warning` | Non-blocking concern with message |
| `ReceiptMeta` | Optional receipt metadata: terminal outcome, retry identity, user-visible evidence |
| `TerminalOutcome` | How a run ended: `Success`, `Failure`, `Timeout`, `Cancelled` |

### Functions

| Function | Description |
| :--- | :--- |
| `set_author(author: String)` | Set global author for envelope metadata. Call once at startup. |

### Constructors

| Constructor | Description |
| :--- | :--- |
| `Envelope::success(cli_version, kind, data, warnings, hints)` | Success envelope |
| `Envelope::success_with_tx(cli_version, kind, data, warnings, hints, tx)` | Success envelope with transaction id |
| `Envelope::error(cli_version, err, warnings)` | Error envelope |
| `Envelope::with_receipt(receipt)` | Builder: attach `ReceiptMeta` (consumes and returns the envelope) |

### CLI version ownership contract

`cli_version` is **caller-supplied at construction** — the first argument to every
constructor. It must be the version of the tool that *emits* the envelope
(typically `env!("CARGO_PKG_VERSION")` in the downstream tool's own crate).

Genesis-vibes never injects its own package version: the misleading
genesis-derived `CLI_VERSION` default was removed in the version that
introduced this change. There is no zero-argument constructor that silently
emits genesis's version — pass your own version explicitly.

**Migration path (for tools adopting the new contract):**

1. At each `Envelope::success` / `success_with_tx` / `error` call site, add the
tool's own version as the first argument:

   ```rust
   Envelope::success(env!("CARGO_PKG_VERSION"), EnvelopeKind::Ok, data, vec![], vec![])
   ```

2. For envelope-producing helpers (`GuideOutput::to_envelope`,
`DoctorReport::to_envelope`, `StatusReport::to_envelope`), the `cli_version`
parameter is now required and must be threaded from the caller.
3. Remove any `use genesis::envelope::CLI_VERSION` — the constant no longer
exists.
4. Verify with `cargo test` that serialized envelopes carry the tool's own
version, not genesis-vibes' version.

### Receipt metadata (opt-in)

`Envelope` carries an optional `receipt: Option<ReceiptMeta>` recording the
terminal outcome, retry identity, and user-visible evidence of a command run
(add-aix-eval-loop D1). It is **additive and opt-in**:

- Envelopes constructed without `with_receipt()` serialize **byte-identically**
  to pre-receipt output — the `receipt` key is omitted entirely
  (golden-file tested in `tests/envelope_golden.rs`). No downstream change
  is required.
- `TerminalOutcome` classifies how a run ended: `Success`, `Failure`,
  `Timeout`, or `Cancelled`. Timeout is explicit, not silent failure, and is
  independent of the envelope's `ok` field (a delivered result with a
  timed-out follow-up is representable).
- `attempt: u32` and optional `idempotency_key` make retries of mutating
  commands distinguishable. Which commands qualify is a per-tool policy —
  genesis ships the mechanism, not the policy.
- `evidence: Option<String>` is a verifiable statement of the user-visible
  edge ("file X exists at path Y"), not free-form narrative.

```rust
use genesis::envelope::{Envelope, EnvelopeKind, ReceiptMeta, TerminalOutcome};

let env = Envelope::success(env!("CARGO_PKG_VERSION"), EnvelopeKind::Ok, data, vec![], vec![])
    .with_receipt(ReceiptMeta {
        terminal_outcome: TerminalOutcome::Success,
        attempt: 1,
        idempotency_key: Some("deploy-config".into()),
        evidence: Some("file exists at /tmp/out.txt".into()),
    });
```

---

## guide

**Signature:** `genesis::guide`

CLI scaffold: verbosity, output format, error handling, and command dispatch.

### Key Types

| Type | Description |
| :--- | :--- |
| `Verbosity` | Progressive-disclosure enum: `Quiet`, `Normal`, `Verbose`, `Debug` |
| `CliVerbosity` | Embeddable clap args struct for `-v`/`-vv`/`-vvv` + `-q`/`--quiet` |
| `OutputFormat` | `Human` or `Json` |
| `CliFormat` | Embeddable clap args struct for `--json`/`--human` with auto-detection |
| `Output<T>` | Fluent builder wrapping `Envelope<T>` |
| `ErrorSink` | Error collector with self-healing suggestions |
| `GuideBuilder` | Builder for assembling genesis modules into a CLI |
| `Guide` | Complete CLI runner with formatted output |

### Functions

| Function | Description |
| :--- | :--- |
| `Output::success(msg)` | Create a success output |
| `Output::emit(cli_version, format, verbosity, stdout, stderr)` | Format-dispatching output (JSON or human) |
| `Verbosity::from_verbose_count(u8)` | Canonical clap count to Verbosity mapping |
| `Verbosity::help_footer()` | "Use -v for..." progressive-disclosure hint |

---

## suggestions

**Signature:** `genesis::suggestions`

Self-healing error suggestions and typo detection.

### Key Types

| Type | Description |
| :--- | :--- |
| `Suggestion` | `DidYouMean(String)` or `Fix(String)` with optional footer |
| `SuggestionEngine` | Typo detection engine with configurable threshold |
| `CommandRegistry` | Registry of known commands for suggestion matching |

### Functions

| Function | Description |
| :--- | :--- |
| `Suggestion::fix(hint)` | Create a fix suggestion |
| `Suggestion::footer()` | Optional footer text |
| `SuggestionEngine::new()` | Create engine with default similarity threshold |
| `SuggestionEngine::with_threshold(threshold)` | Create engine with custom similarity threshold |
| `SuggestionEngine::suggest_typo(unknown, registry)` | Find closest match for an unknown command |
| `CommandRegistry::register(tool, commands)` | Register commands for a tool |

---

## managed_block

**Signature:** `genesis::managed_block`

Managed block injector for `<!-- NAME:START -->` / `<!-- NAME:END -->` markers in markdown files.

### Key Types

| Type | Description |
| :--- | :--- |
| `BlockDef` | Named block with auto-generated or custom markers |
| `BlockInjector` | Injects, reads, and detects content within managed blocks |
| `BlockRegistry` | Collection of `BlockDef` entries |
| `InjectResult` | `Injected`, `Updated`, `NoChange`, `BlockNotFound` |

### Functions

| Function | Description |
| :--- | :--- |
| `BlockDef::new(name)` | Create block with auto-generated `<!-- NAME:START/END -->` markers |
| `BlockDef::with_markers(name, start, end)` | Create block with custom markers |
| `BlockInjector::inject(path, block_name, content)` | Inject or update content in a managed block |
| `BlockInjector::has_block(path, block_name)` | Check if a block exists in a file |
| `BlockInjector::read_block(path, block_name)` | Read the current content of a block |

---

## config

**Signature:** `genesis::config`

Shared config management with validation and error reporting.

### Key Types

| Type | Description |
| :--- | :--- |
| `ConfigFile` trait | `read()`, `write()`, `validate()` for tool config files |
| `ConfigRegistry` | Tool registration for config file discovery |
| `ConfigError` | `MissingFile`, `ParseError`, `ValidationError`, `TypeMismatch` |
| `ConfigValidation` | Validation result with `field`, `message`, `severity` |
| `ValidationSeverity` | `Warning` or `Error` |

### Functions

| Function | Description |
| :--- | :--- |
| `ConfigFile::read()` | Read and parse config from the default path |
| `ConfigFile::write()` | Write config to the default path |
| `ConfigFile::validate()` | Validate config contents |
| `ConfigRegistry::register<T>(tool_name)` | Register a config file type for a tool |
| `ConfigError::to_suggestion()` | Convert error to a user-facing `Suggestion` |

---

## aix

**Signature:** `genesis::aix`

AIX artifact generation helpers for `llms.txt`, `llm.txt`, and `AGENTS.md` blocks,
plus token-cost estimation and budget-bounded generation (add-aix-eval-loop §2).

### Key Types

| Type | Description |
| :--- | :--- |
| `ProjectMeta` | Project name, tagline, repository/documentation/crates.io links |
| `ModuleEntry` | One module listing: name + description |
| `LlmSection` | `llm.txt` section: `Heading`, `Table`, or `Raw` |
| `TokenCost` | Heuristic token estimate + the heuristic's name (`chars/4`) |

### Functions

| Function | Description |
| :--- | :--- |
| `generate_llms_txt(meta, modules)` | Generate `llms.txt` from project metadata and modules |
| `generate_llm_txt(title, description, sections)` | Generate `llm.txt` from sections |
| `generate_llms_txt_bounded(meta, modules, budget)` | `llms.txt` under a token budget; deterministic degradation ladder |
| `generate_llm_txt_bounded(title, description, sections, budget)` | `llm.txt` under a token budget; deterministic degradation ladder |
| `estimate_token_cost(s)` | chars/4 token estimate (ceiling division), labeled with the heuristic name |
| `agents_block(name, body)` | Generate an agent block with body content |

### Token-cost heuristic

`estimate_token_cost` uses the **chars/4 heuristic** (ceiling division) with a
documented **±25% error band** against typical English prose in cl100k-class
vocabularies — good enough for budget arbitration, not for billing. The
heuristic name ships inside every `TokenCost` so an approximation is never
mistaken for a tokenizer count. No tokenizer dependency is pulled in.

### Budget degradation ladder

Both bounded generators degrade **deterministically** instead of overflowing
or refusing. Granularity steps, in order:

1. at or under budget → byte-identical to the unbudgeted generator
2. truncate descriptions to their first sentence
3. drop optional (`Raw`) sections
4. drop table content — headings always survive; module names are the floor

The floor artifact is returned even if it still exceeds the budget: a verbose
artifact beats a missing one. Existing `generate_llms_txt` / `generate_llm_txt`
signatures are untouched (golden-file pinned in `tests/aix_bounded.rs`).

---

## evals

**Signature:** `genesis::evals`

Deterministic evaluation harness: scenarios replayed against fake-agent
transcripts, with envelope-assertion helpers, reusable checks, distractor
fixtures, and the doc-drift blindness check (add-aix-eval-loop §3). No live
LLM, no subprocess runner.

### Key Types

| Type | Description |
| :--- | :--- |
| `AgentStep` | One replayed agent turn: command + captured output |
| `Scenario` | Fixture setup + prompt + deterministic checks (builder) |
| `ScenarioResult` | Replayed trajectory + fixture root + distractor registry |
| `ScenarioReport` | Replay outcome; serializable; optional `model` attribution |
| `CheckOutcome` | `Pass` or `Fail { taxonomy, reason }` |
| `ErrorTaxonomy` | Closed `ERR_*` failure classification |
| `DistractorKind` | `StaleDocs` or `ContradictingHint` |
| `Distractor` | Bait file materialized in the replay environment |
| `EnvelopeOutcome` | Lenient parse of captured stdout (`Ok` / `Error`) |

### Functions

| Function | Description |
| :--- | :--- |
| `Scenario::new(name, prompt)` | Start building a scenario |
| `Scenario::fixture_file(path, content)` | Add a task fixture file |
| `Scenario::distractor_file(path, content, kind)` | Add a distractor (bait) file |
| `Scenario::check(name, check)` | Add a deterministic check |
| `Scenario::run(replay)` | Materialize fixture, replay steps, apply checks |
| `ScenarioReport::with_model(model)` | Attribute the replay to a model (matrix runs) |
| `parse_envelope(stdout)` | Lenient envelope parse (only `ok` required) |
| `error_envelope_with_hint(step, cmd)` | Check: failing step carries the hint |
| `ok_envelope(step)` | Check: step recovered with `ok: true` |
| `agent_followed_hint(index, cmd)` | Check: agent ran the suggested fix |
| `agent_executed_all()` | Check: no hallucinated steps |
| `doc_drift_blindness(bait)` | Check: envelope trusted over stale docs |

### Distractors and doc-drift blindness

Distractor files share the replay environment with real fixtures but are
registered separately, so checks can tell bait from task material. Presence
of a distractor never faults a run by itself — the check decides:

- `doc_drift_blindness(bait_cmd)` passes when the agent received a parseable
  envelope from an executed step and never issued `bait_cmd` (the action only
  correct per the stale docs). Action-level correctness composes with
  `ok_envelope` / `agent_followed_hint`.
- Doc-following steps fail as agent fault `ERR_DOC_DRIFT_BLINDNESS` with the
  distractor path in the reason.
- A scenario without a `StaleDocs` distractor makes the check a **tool fault**
  (misconfiguration), not an agent fault.

`ScenarioReport` serializes to JSON (name, passed, failures with taxonomy
codes, `model` when attributed) for per-model matrix comparison; `run()`'s
signature is unchanged — attribution happens post-hoc via `with_model`.

---

## feedback

**Signature:** `genesis::feedback`

Agent issue reporting — wraps scratch (error persistence), context (env bundle),
redactor (privacy), and gh (GitHub issue creation) into a single command.

### Key Types

| Type | Description |
| :--- | :--- |
| `FeedbackArgs` | `kind`, `dry_run`, `from_last_error` |

### Functions

| Function | Description |
| :--- | :--- |
| `handle_feedback(args)` | Run the feedback workflow: collect context, redact, create issue |

### Sub-modules

| Module | Purpose |
| :--- | :--- |
| `feedback::context` | Environment bundle collection |
| `feedback::gh` | GitHub issue creation via `gh` CLI |
| `feedback::redactor` | Privacy redaction |
| `feedback::scratch` | Error persistence from previous runs |

---

## suite_linter

**Signature:** `genesis::suite_linter`

Suite-wide config lint checks. Foundation for the `doctor` module.

### Key Types

| Type | Description |
| :--- | :--- |
| `LintCheck` trait | `name()`, `check(repo)` for a single lint rule |
| `LinterRegistry` | Collection of `LintCheck` instances with batch execution |
| `LintResult` | Single lint finding with `message`, `severity`, optional `fix` |
| `Severity` | `Error`, `Warning`, `Info`, `Hint` |

### Functions

| Function | Description |
| :--- | :--- |
| `LintResult::new(message, severity)` | Create a lint finding |
| `LintResult::with_fix(message, severity, fix)` | Create a lint finding with auto-fix hint |
| `LinterRegistry::register(check)` | Register a lint check |
| `LinterRegistry::run_all(repo_root)` | Run all registered checks |
| `LinterRegistry::run_named(name, repo_root)` | Run a single check by name |
| `LintCheck::check(repo)` | Run the check and return findings |

---

## fixture

**Signature:** `genesis::fixture`

Test scratch environments and runners.

### Key Types

| Type | Description |
| :--- | :--- |
| `Fixture` | Builder for temp directories with markers, files, git init |
| `FixtureError` | `EmptyCommand`, `Spawn`, `Git`, `Io`, `Serde` |

### Fixture builder methods

| Method | Description |
| :--- | :--- |
| `Fixture::new()` | Create a new fixture builder |
| `.with_marker(path)` | Create a marker directory/file |
| `.with_file(path, content)` | Write a file with content |
| `.with_toml(path, value)` | Write a serializable struct as TOML |
| `.with_git_init()` | Initialize a git repo |
| `.build()` | Build the fixture (returns `Fixture`) |
| `.run(program, args)` | Run a command in the fixture directory |

---

## doctor

**Signature:** `genesis::doctor`

Diagnostic framework with auto-fix.

### Key Types

| Type | Description |
| :--- | :--- |
| `DoctorCheck` trait | `name()`, `description()`, `run()`, `can_fix()`, `fix()` |
| `DoctorRunner` | Runs a collection of checks with `run(repo, fix)` |
| `DoctorReport` | Structured result with `summary` and `results` |
| `CheckStatus` | `Pass`, `Warn`, `Fail` |

---

## cli

**Signature:** `genesis::cli`

CLI helpers.

### Functions

| Function | Description |
| :--- | :--- |
| `generate_completions()` | One-liner for clap_complete shell completions |
| `maybe_print_version_json(name, version)` | Pre-parse `--version --json` before clap |

---

## status

**Signature:** `genesis::status`

Cross-tool status dashboard.

### Key Types

| Type | Description |
| :--- | :--- |
| `StatusContributor` trait | `name()`, `status()` |
| `StatusBuilder` | Aggregates all contributors into a unified report |
| `StatusLevel` | `Healthy`, `Warning`, `Error`, `Unknown` |
| `StatusSection` | Named section with items and level |
| `DoctorStatusBridge` | Wraps any `DoctorRunner` as a `StatusContributor` |

---

## scaffold

**Signature:** `genesis::scaffold`

Init scaffolding builder.

### Key Types

| Type | Description |
| :--- | :--- |
| `Scaffold` | Builder for directories, configs, gitignore, managed blocks |
| `ScaffoldResult` | `created: Vec<PathBuf>`, `existed: Vec<PathBuf>` |

### Scaffold builder methods

| Method | Description |
| :--- | :--- |
| `Scaffold::new(path)` | Create a new scaffold for a project path |
| `.dir(dir)` | Create a directory |
| `.default_config(path, content)` | Write a default config file |
| `.gitignore_entry(pattern)` | Add a `.gitignore` entry |
| `.managed_block(name, content)` | Inject a managed block |
| `.agent_command_file(path, content)` | Create an agent command file |
| `.build()` | Build the scaffold (returns `ScaffoldResult`) |

---

## discovery

**Signature:** `genesis::discovery`

Tool discovery via `.genesis/tools.toml` manifest.

### Key Types

| Type | Description |
| :--- | :--- |
| `Manifest` | TOML manifest with `tools` table |
| `DetectedTool` | `name`, `description`, `detected` (bool), `detector_type` |

### Functions

| Function | Description |
| :--- | :--- |
| `scan(project)` | Scan for all registered tools |
| `register(project, name, desc, type, path)` | Register a tool |
| `unregister(project, name)` | Remove a tool registration |
| `list_tools(project)` | List all registered tools |
| `has_manifest(project)` | Check if manifest exists |