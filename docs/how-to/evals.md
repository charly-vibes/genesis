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

Prefer these checks, in order of reliability:

| Check | How | Example |
|---|---|---|
| Envelope assertions | Parse the captured stdout JSON | `envelope.ok == false`, `envelope.hints` non-empty and mentioning the suggested fix |
| Exit codes | Subprocess status | `0` success vs. the tool's error code — note the current contract is coarse: `guide::run` exits `1` for *every* user-facing error (no gradation), and panics unwind to Rust's default `101` with no documented guarantee. Track [genesis-u40](https://github.com/charly-vibes/genesis) for a refined exit-code contract; until then, treat "exited nonzero with a parseable error envelope" as the portable assertion |
| Managed-block boundary audits | Line-level diff of `AGENTS.md` | Changes occur only between `<!-- my-tool:START -->` and `<!-- my-tool:END -->` |
| State-transcript diffs | Parse your tool's state/log files | Transitions obey the state machine; no direct hand-edits of state files |

## Step 4: Classify failures with a taxonomy

When a trial fails, assign an error code from a fixed vocabulary so results
aggregate into dashboards instead of pass/fail noise:

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

## What genesis provides today (and what it doesn't)

**Available now:** `Fixture` for sandboxing, `envelope` for structured output your
assertions parse, `suggestions` for `DidYouMean` / `Fix` payloads, `managed_block`
markers for boundary audits, `doctor` with auto-fix as a recovery target.

**Gaps worth tracking:** no `evals` module yet — scenario specs, transcript-capture
and envelope-assertion helpers, managed-block audit utilities, the error taxonomy
enum, and AIX-ablation provisioning are all hand-rolled today ([genesis-zxv](https://github.com/charly-vibes/genesis)).
The exit-code contract does not yet distinguish user errors from internal panics,
which limits exit-code-based assertions ([genesis-u40](https://github.com/charly-vibes/genesis)).
