# Creating Agent Evals for Your Tool

**TL;DR:** Build evals that check whether LLM agents *use* your tool correctly — not
whether they can write code. Run scenarios in sandboxed fixtures against real
subprocesses, assert on the machine-checkable signals your tool already emits
(envelopes, exit codes, state transcripts, managed blocks), and classify every
failure into a taxonomy that tells you what to fix.

## Context & Prerequisites

This guide explains how to author *agent evaluations* — scenarios that measure
whether an autonomous agent reads, parses, and acts on your tool's output channels.
It is different from unit testing: a unit test checks your binary against inputs you
control; an eval checks an *agent's behavior* against a task you've framed.

Before starting, ensure you have:

- Added `genesis-vibes` to your `Cargo.toml`
- Read [Writing Tests with Fixture](fixture.md) — fixtures are the eval sandbox
- Read [Using the Envelope](envelope.md) — envelopes are the primary assertion target

## Why evals differ from tests

Your tool communicates with agents through structured channels: the JSON envelope
(`ok`, `warnings`, `hints`, `data`), self-healing suggestions (`DidYouMean` / `Fix`),
managed blocks in `AGENTS.md`, and discovery artifacts (`llms.txt`,
`.genesis/tools.toml`). An agent can pass a task while ignoring every one of these
channels — brute-forcing until something works — or fail a task while behaving
flawlessly. A good eval battery measures **protocol adherence** separately from task
success, because each failure mode points at a different fix: hint-blindness means
your output channel needs work; task failure with perfect adherence means the task
was misframed.

The core discipline: **never score free-form agent text.** Score only what crossed
the process boundary — subprocess exit codes, exact stdout/stderr envelopes, and
filesystem diffs. This eliminates the tool-hallucination confound, where a model
narrates plausible tool output without ever running the command.

## The eval formula

Every eval scenario is four things:

1. **Fixture** — the initial sandbox state
2. **Agent prompt** — what the agent is asked (framed at a difficulty tier)
3. **Deterministic checks** — assertions over envelopes, exit codes, and file state
4. **Error code** — what a failure means, in taxonomy terms

## Step 1: Provision the sandbox

*(Elaborates the **Fixture provisioning** and **Sandbox confinement** requirements
of the `evals-guidelines` spec: the tool binary is provisioned before the agent's
first turn; live runs use an isolated `HOME`, a scrubbed environment, network
denied by default, and a per-command timeout.)*

Use `Fixture` for the scratch environment. The tool binary must be pre-built and on
`PATH` *before* the agent's turn — compiling from source inside a trial burns budget
and measures build skills, not tool comprehension. (Installation is only in-scope if
the scenario itself is about installation.)

Two safety rules for the sandbox: no network access unless the scenario explicitly
requires it, and run the agent with an isolated `HOME`/environment (a live agent in
a temp directory can still reach the real user home unless the harness blocks it).

Driving the agent depends on the cadence. For **CI regression** (every commit,
zero-cost), use a mock or replay agent: a script that replays a recorded trajectory
or a stub agent that follows a simple decision rule — the deterministic checks are
the value, not the model. For **external capability runs** (nightly/scheduled,
multi-model), use a harness like Inspect AI with per-trial container sandboxing and
multi-provider model support.

```rust
use genesis::fixture::Fixture;

let fixture = Fixture::new()
    .with_marker(".genesis")
    .with_file(".genesis/tools.toml", "unknown_key = \"invalid\"\n")
    .with_file("AGENTS.md", "<!-- my-tool:START -->\nexisting context\n<!-- my-tool:END -->\n")
    .build()
    .expect("build fixture");
```

If the scenario depends on AIX artifacts, provision them explicitly — and see
[Step 5](#step-5-ab-your-aix-artifacts) for why you also want the *ablated* variant.

## Step 2: Inject contrived failures

*(Elaborates the **Scenario provenance** requirement of the `evals-guidelines`
spec: every scenario traces to an observed failure source — a corpus entry, an
error-analysis note, or a ticket. Contrived-failure scenarios are declared as
channel stress tests; their results bound detection claims, never prevalence
claims.)*

Your tool's self-healing output (hints, suggestions, `doctor --fix`) is a promise:
*errors help the agent recover*. Evals are where you prove it. Deliberately trigger
error paths and check whether the agent's next action consumes the payload:

- **Flag perturbation** — invoke with a mistyped subcommand; expect the agent's next
  call to use the `DidYouMean` suggestion.
- **Invalid state** — place the environment in an illegal state (e.g., a `dont`-style
  state machine mid-transition); expect the agent to follow the remediation in the
  hint envelope rather than editing state files directly.
- **Corrupt config** — malformed `.genesis/tools.toml`; expect the agent to run
  `doctor` / `doctor --fix` within a step or two of receiving the hint.

These are the highest-value scenarios in any battery: they test the exact value
proposition of the AIX investment.

## Step 3: Assert deterministically

*(Elaborates the **Process-boundary scoring**, **One fault per check**, and
**Empty-trajectory guard** requirements of the `evals-guidelines` spec: assert
only on signals that crossed the process boundary; one fault classification per
check; a passive model cannot pass by doing nothing.)*

Prefer these checks, in order of reliability:

| Check | How | Example |
|---|---|---|
| Envelope assertions | Parse the captured stdout JSON | `envelope.ok == false`, `envelope.hints` non-empty and mentioning the suggested fix |
| Exit codes | Subprocess status | `0` success, `1` user-facing error (graceful failure — parse the error envelope for hints), `2` internal failure (I/O while emitting output); panics unwind with Rust's default behavior (typically `101`) and are never masked. Contract documented on `Guide::run` ([genesis-u40](https://github.com/charly-vibes/genesis)). Assert `1` for "graceful failure with hint envelope" vs. any other nonzero for "crashed" |
| Managed-block boundary audits | Line-level diff of `AGENTS.md` | Changes occur only between `<!-- my-tool:START -->` and `<!-- my-tool:END -->` |
| State-transcript diffs | Parse your tool's state/log files | Transitions obey the state machine; no direct hand-edits of state files |

## Step 4: Classify failures with a taxonomy

When a trial fails, assign an error code from a fixed vocabulary so results
aggregate into dashboards instead of pass/fail noise. The codes and the
agent/tool fault vocabulary they ride on are exactly what the
[report contract](../reference/eval-report.md) serializes:

- `ERR_ENVELOPE_HINT_BLINDNESS` — the tool returned `ok: false` with hints; the
  agent's next command ignored the suggested fix
- `ERR_STATE_MACHINE_VIOLATION` — illegal transition, or bypassed state checks by
  hand-editing files
- `ERR_MANAGED_BLOCK_CORRUPTION` — agent overwrote or deleted managed-block markers
- `ERR_TOOL_EXECUTION_HALLUCINATION` — agent narrated a command it never executed
  (absent from the subprocess log)
- `ERR_CONTEXT_RECOVERY_FAILURE` — agent hit an unexpected error but never ran the
  ecosystem's orientation commands (`wai prime` / `wai status` for context recovery,
  `doctor` for diagnostics)
- `ERR_TOOL_DISCOVERY_FAILURE` — agent defaulted to a generic approach, never
  discovering the specialized tool
- `ERR_ACTION_FORMAT_VIOLATION` — live-tier agent failed to produce a parseable
  structured action after one re-ask (the re-ask consumes a turn). Distinct
  from `ERR_TOOL_EXECUTION_HALLUCINATION`, which stays a replay-tier code for
  claims of runs that never happened — in the live tier the harness executes
  every action, so that failure mode is impossible there.

## Step 5: A/B your AIX artifacts

Run every scenario twice: **full** (with `llms.txt`, managed `AGENTS.md` guidance,
`.genesis/tools.toml`) and **ablated** (raw binaries, no AIX context). The score
delta *is* the measured value of your documentation and signaling investment. If a
hint channel shows no delta, it is either unread, unparsable, useless — *or your
scenarios are too easy for the agent to need it* (or your sample count is too small
to detect the delta). All five are actionable findings; the last two mean fix the
scenario, not the tool.

## Step 6: Run across model tiers

The same scenario should discriminate across capability tiers. Knobs that scale
difficulty without changing the scenario:

- **Prompt specificity** — easy tiers get exact subcommands; hard tiers get intent
  only ("resolve the failing claim"), forcing discovery via `--help` / `llms.txt`
- **Distractors** — noisy logs, unused config files, decoy tools in the registry
- **Perturbation for contamination defense** — the suite's docs (`llms.txt`, mdBook,
  crates.io) are public and likely in model training data, so a frontier model may
  be *recalling* syntax rather than comprehending it. Rename flags/subcommands in
  the sandbox (e.g., `conclude` → `finalize`) and require discovery via the local
  `llms.txt`; keep a set of private held-out fixtures that have never been published
- **Step and token caps** — budgets enforced by the harness; trajectories exceeding a
  step limit without progress are terminated early
- **Sample counts** — agentic runs are stochastic; run n ≥ 3 trials per scenario per
  model and report pass@k and variance, never single-run pass/fail

## Anatomy of a complete scenario

> **Illustrative sketch.** The `transcript` API below does not exist yet — it is
> the shape the planned evals module ([genesis-zxv](https://github.com/charly-vibes/genesis))
> will provide. Write your checks against captured subprocess logs directly today.

```rust
// evals/hint_adherence.rs — sketch
let fixture = Fixture::new()
    .with_file(".genesis/tools.toml", "unknown_key = \"invalid\"\n")
    .build()?;

// 1. Agent turn (via your harness): "Run diagnostics and resolve any errors."
//    Harness captures every subprocess call: argv, exit code, stdout.

// 2. Deterministic checks over the transcript:
let calls = transcript.calls();
assert!(calls.iter().any(|c| c.envelope_failed() && c.hints_mention("doctor --fix")),
    "tool must surface a self-healing hint for invalid config");
assert!(calls.iter().any(|c| c.argv.starts_with("my-tool doctor")),
    "agent should run the hinted fix — else ERR_ENVELOPE_HINT_BLINDNESS");

// 3. Post-fix state:
let final_env = run(&fixture, "my-tool doctor");
assert_eq!(final_env.exit_code, 0);
assert!(final_env.envelope.ok);
```

## Step 7: Plant distractors and test doc-drift blindness

*(Elaborates the **Distractor registration** requirement of the
`evals-guidelines` spec: distractors are materialized into the replay
environment but are not task material — whether consulting them is a fault is
the check's decision, never their presence.)*

A frontier model that *recalls* your public docs will trust them over your tool's
live output — the most expensive failure mode in agentic use. genesis ships the
mechanism for testing exactly this (add-aix-eval-loop §3):

```rust
use genesis::evals::{doc_drift_blindness, AgentStep, DistractorKind, Scenario};

let scenario = Scenario::new("doc-drift", "Initialize my-tool")
    // Bait: materialized in the sandbox, registered as stale docs.
    .distractor_file(
        "AGENTS.md",
        "<!-- my-tool:START -->\nRun `my-tool configure` to initialize.\n<!-- my-tool:END -->\n",
        DistractorKind::StaleDocs,
    )
    .check("doc-drift", doc_drift_blindness("my-tool configure"));

// Agent read --help and followed the envelope → pass.
// Agent ran the stale `configure` instead → agent fault
// ERR_DOC_DRIFT_BLINDNESS with the distractor path in the reason.
```

Distractors never fault a run by themselves — presence is not fault, the check
returns the verdict. Compose `doc_drift_blindness` with `ok_envelope` /
`agent_followed_hint` for action-level assertions, and attribute replays to models via
`ScenarioReport::with_model` for matrix comparison.

## Step 8: Run the tier ladder

*(Elaborates the **Tier ladder** and **Battery maintenance** requirements of the
`evals-guidelines` spec — `openspec/specs/evals-guidelines/spec.md`.)*

Every consumer battery runs three tiers. Each tier states what it may claim —
citing a tier for more than it measures is the failure the ladder exists to
prevent:

| Tier | What runs | Cadence | May claim |
|---|---|---|---|
| 0 — static | No model: schema checks, envelope shape, doc-sync guards | Every push | "the channels are well-formed" |
| 1 — scripted replay | `Scenario::run` over recorded `AgentStep` transcripts | Nightly | "recorded failure modes are still detected" |
| 2 — live model | Your harness runs a real model per the action protocol below | Scheduled, rotated across model ids | "agents currently behave this way" — directional only |

Live trials never gate a push: they are directional evidence, not CI gates. A
permanently green battery is **never** evidence of quality — see battery
maintenance below.

### The live action protocol

Your tier-2 harness requires the agent to return **one structured action per
turn** — the exact command line plus a done flag. Per turn the model receives
only: the scenario prompt and the fixture root path inside the sandbox. Never
the check logic, the expected outcomes, or the distractor registry.

1. Malformed output → re-ask **once**, with the format error attached; the
   re-ask consumes a turn of the cap.
2. Malformed again → trial ends `invalid_output`, agent fault
   `ERR_ACTION_FORMAT_VIOLATION`.
3. Parseable action → execute the command **exactly once**, record the step
   with `executed: true`.

### Bounds

Every live trial is bounded by a **turn cap**, a **wall-clock timeout**, and a
**token-estimate budget** (the `aix` chars/4 heuristic over the trial's full
prompt and completion traffic — labeled as heuristic). The applied values are
recorded in the report. A trial stopped at a bound ends `failed` and names the
reached bound; the stop itself is **not** attributed as an agent or tool fault.

### Free-tier entry and attribution

Use OpenRouter `:free` model ids as the entry tier — the weakest readers. If a
weak model parses the envelope and honors the hint, the channel is robust
([why](../explanation/why-weak-readers.md)). Record the raw id **verbatim**
(`deepseek/deepseek-v4-flash:free`, suffix included) — matrix comparisons across
time depend on the unnormalized id. Run n ≥ 3 repetitions per scenario × model;
each repetition is a distinct report row. On HTTP 429: retry once after ≥ 2 s
(record the delay used); a second 429 ends the trial `rate_limited` — never a
tool fault, never a silent substitution of a different model.

### Battery maintenance (staleness review)

Checks that never fail are rotting, not succeeding. Review the battery on a
cadence: any check that has not failed since its last review is either
re-sharpened (harder distractor, tighter envelope assertion) or retired with a
note. Declare contrived-failure scenarios as **channel stress tests** — their
results bound detection claims, never prevalence claims ("we detect X when it
happens", never "X happens at rate Y").

## Fault routing

*(Elaborates the **Fault routing** requirement of the `evals-guidelines` spec.)*

A failure is a **tool fault** or an **agent fault**, never both:

- **Tool faults** (the tool misbehaved — envelope missing, exit code wrong)
  become tickets in your tool's repo. They are product bugs wearing eval
  clothes.
- **Agent faults** (taxonomy-classified, `ERR_*` code attached) are routed by
  check code × output channel across ≥ 3 model ids (or all configured ids when
  fewer). A pattern that reproduces across models points at **your output
  channel** (AIX work in genesis), not at the model. Sub-threshold occurrences
  are recorded without action.

## Reporting and aggregation

Every trial emits one report row in the interoperable JSON contract — see
[the eval report contract](../reference/eval-report.md) and the normative
fixtures in `tests/golden/eval_report_*.json`. Dashboards aggregate rows by
`report_version` with no per-tool mapping.

## What genesis provides today (and what it doesn't)

**Available now:** `Fixture` for sandboxing, `envelope` for structured output your
assertions parse, `suggestions` for `DidYouMean` / `Fix` payloads, `managed_block`
markers for boundary audits, `doctor` with auto-fix as a recovery target, and the
`evals` module for deterministic scenario replay: `Scenario::run` over recorded
`AgentStep` transcripts, envelope-assertion helpers (`parse_envelope`,
`ok_envelope`, `error_envelope_with_hint`, `agent_followed_hint`,
`agent_executed_all`), the `ErrorTaxonomy` classification, distractor fixtures
with the `doc_drift_blindness` check, and feedback→Scenario conversion so
user-reported failures become regression scenarios
(see [reference/modules.md](../reference/modules.md)).

**Gaps worth tracking:** live-agent harness capture (running a real model and
recording its steps) stays outside the crate — `Scenario::run` replays recorded
transcripts deterministically; orchestrate live runs and per-model matrices in
your harness. AIX-ablation provisioning helpers remain a follow-up slice
([genesis-zxv](https://github.com/charly-vibes/genesis)).

## Related

- **Token budgets for LLM consumption:** `estimate_token_cost` and the bounded
  generators (`generate_llms_txt_bounded` / `generate_llm_txt_bounded`) degrade
  artifacts deterministically instead of overflowing a declared budget —
  useful when provisioning context for tier-limited models.
