# Deep Research Prompt: Evals for Agentic CLI Tool Usage

> **How to use:** hand everything below **The Prompt** heading to a deep-research
> agent (an OpenAI or Gemini deep-research product, or a `wai research` session).
> It is self-contained: the researcher needs no prior knowledge of the charly-vibes
> suite. When the report comes back, save it to `research/findings/<date>-cli-agent-evals.md`.

---

## The Prompt

You are a research agent tasked with producing an evidence-grounded report on **how to
build an evaluation suite for AI agents that use command-line developer tools**. The
report will drive the creation of real evals for a suite of Rust CLIs whose primary
"users" are increasingly LLM agents, not humans.

### 1. The system under evaluation

The **charly-vibes** suite is a family of CLI tools plus a shared Rust crate
(`genesis-vibes`) that provides cross-cutting agent-experience (AIX) infrastructure.
The tools are designed to be operated *by coding agents* inside autonomous workflows:

- **`genesis-vibes`** (shared crate): structured JSON output envelopes (`ok`,
  `envelope_version`, `data`, `warnings`, `hints`), self-healing error suggestions
  (`DidYouMean` / `Fix`), managed-block injection into `AGENTS.md`, AIX artifact
  generation (`llms.txt`, `llm.txt`), config registries, doctor diagnostics with
  auto-fix, scaffolding, tool discovery via `.genesis/tools.toml`, and agent feedback
  collection (redaction, context bundles, `gh` issue invocation).
- **`dont`** — epistemic discipline: agents record claims (`dont conclude`), verify
  with evidence (`dont flag`), and pass through an explicit claim state machine.
- **`wai`** — context recovery and workflow management: captures *why* decisions were
  made; `wai prime` / `wai status` orient an agent in a project.
- **`espectacular` (`ah`)** — behavioral verification / spec-test correspondence.
- **`pretender`** — structural code-quality checks (complexity, duplication, mutation).
- **`testaruda`** — language-agnostic test selection from a code change.

Key design intent: the tools **communicate with agents deliberately** — via JSON
envelopes, hints, suggestions, managed blocks, and `AGENTS.md` instructions. Whether
agents actually *read, parse, and act on* these channels is exactly what the evals
must measure.

### 2. Research questions

Answer each with citations to concrete prior art (papers, benchmark repos, harness
docs, blog posts with methodology detail). Prefer primary sources over summaries.

**RQ1 — Landscape of agentic CLI/terminal evals.**
What exists today? Cover at minimum: SWE-bench (and variants), Terminal-Bench /
Harbor, OSWorld, InterCode, Aider's benchmarks, commit0, and any harnesses built for
evaluating *tool-use correctness* rather than end-task completion. For each: what it
measures, how it scores, sandboxing model, and known criticisms (saturation,
contamination, judge fragility).

**RQ2 — Measuring "does the agent understand the library" vs "does the agent finish
the task."** What methodologies distinguish *comprehension* (parsing envelopes,
honoring hints, following managed-block protocol, respecting state machines like
`dont`'s claim lifecycle) from *task success*? Look for: protocol-adherence testing,
capability probing, contrived-failure injection (does the agent notice when the tool
returns `ok: false` with a hint?), and reading-comprehension-style evals over
documentation (`llms.txt`).

**RQ3 — Scoring and judging.** When should scoring be deterministic (parse the JSON
envelope the agent produced; check exit codes; diff the state machine) vs LLM-judged
(rubric-based grading of trajectory quality)? What are the documented failure modes
of LLM-as-judge for agentic trajectories, and what mitigations exist (pairwise
comparison, evidence-anchored rubrics, judge ensembles, judge-model decoupling)?

**RQ4 — Differentiating across model capability tiers.** The suite owner wants to run
the same eval battery against frontier models and weaker/cheaper ones and get a
meaningful spread — not saturation at the top or a floor at the bottom. What
techniques produce graded difficulty: task decomposition, budget constraints (token
/ step limits), distractor injection, longer horizons, ambiguous instructions,
partial-information settings? How do existing benchmarks report per-tier results and
avoid single-number collapse? Also address **contamination specific to this suite**:
its documentation (`llms.txt`, mdBook docs on GitHub Pages, crates.io READMEs) is
public and likely in frontier-model training data, so a "comprehension" eval may be
measuring memorization. What mitigation techniques exist (held-out scenarios,
private fixtures, post-cutoff tool versions, perturbed flags/commands) and how well
are they proven to work?

**RQ5 — Evals that *report improvements*.** The owner wants evals whose output is
actionable: not just pass/fail, but "agents consistently miss the `hints` field in
envelope output" or "weaker models ignore `dont trust` before asserting." Survey
approaches for turning eval trajectories into structured improvement feedback:
failure-taxonomy clustering, error-mode dashboards, regression suites per error mode,
and evals embedded in CI for the tools themselves (dogfooding).

**RQ6 — Harness and infrastructure.** Compare existing harnesses for feasibility:
Inspect AI, promptfoo, Braintrust, Harbor/terminal-bench harness, LangSmith, or a
custom Rust harness reusing `genesis`'s own `fixture` and `envelope` modules (the
suite could dogfood its eval infrastructure). Criteria: multi-model support, sandbox
per trial, deterministic replay, cost controls, CI integration.

### 3. Constraints and context

- Tools are Rust binaries installed via cargo/homebrew; evals must run them as real
  subprocesses (no in-process mocking) to count as agentic evals.
- Every trial must be sandboxed: fresh temp directory via a fixture mechanism, no
  network unless the scenario requires it, no access to the real user home.
- The suite already emits machine-checkable signals (envelope JSON, exit codes,
  state-machine transcripts in `dont`, managed-block content in `AGENTS.md`). Evals
  should prefer checking these artifacts over judging free-form prose.
- Eval battery must run in CI against the tools themselves (regression detection)
  *and* against external models (capability assessment) — two different cadences.
- Trial setup must not spend the budget on tool installation: assume pre-built
  binaries (cargo install of a published version, or a bundled release artifact)
  are provisioned into the sandbox *before* the agent's turn begins. Installing
  from source is only in-scope if the eval scenario itself is about installation.
- Agentic runs are stochastic: the design must specify per-scenario sample counts,
  and report variance and pass@k — not single-run pass/fail.
- Budget: assume eval runs must complete for <$5 per full battery on cheap models;
  flag anything in the design that breaks this.
- The research report itself should be ≤5000 words, with a comparison table of prior
  art, full URLs for every citation, and must clearly separate *established
  practice* from *your extrapolation*.

### 4. Deliverables

1. **Landscape table** — prior art mapped against RQ1–RQ6 with links.
2. **Recommended eval taxonomy** for the suite, with 3–5 concrete example eval specs.
   Each spec must include: task setup, agent-facing prompt, environment state,
   deterministic checks, judge rubric (if any), difficulty knobs for model tiers, and
   the improvement signal it produces on failure.
3. **Harness recommendation** — build on existing vs custom, with justification tied
   to the constraints above.
4. **Grading-across-tiers strategy** — how to make results interpretable from a
   <100B open model to a frontier model.
5. **Roadmap** — a phased plan (smallest useful eval set first) with an estimate of
   effort per phase.
6. **Open questions** — what could not be answered from prior art and needs
   experimentation.
7. **Citation appendix** — every claim in the report must carry a full-URL citation;
   no uncited assertions in the established-practice sections.

### 5. Anti-goals

- Do not propose fine-tuning or training-time interventions; this is evaluation only.
- Do not propose evals that require the agent to have the suite's source code in
  context — the evals measure *black-box* behavior with the tools installed and the
  AIX artifacts (`llms.txt`, `AGENTS.md`) present, mirroring real usage.
- Do not collapse everything into one aggregate score; per-capability breakdown is a
  hard requirement.
- Do not recommend an approach that cannot distinguish "agent used the tool
  correctly" from "agent hallucinated plausible tool output" — this confound is the
  single most important thing to design against.

---

## Notes for the suite owner (not part of the prompt)

- **Why contrived-failure injection matters here:** the suite's bet is that
  self-healing output (hints, suggestions, doctor) improves agent behavior. That
  claim is currently untested — evals that deliberately trigger error paths (unknown
  subcommand, invalid state transition, malformed config) and measure whether the
  agent *uses* the suggestion are the highest-value evals in the whole battery.
- **A/B the AIX artifacts:** run the same scenario with and without `llms.txt` /
  `AGENTS.md` present in the sandbox. The delta *is* the measured value of the AIX
  investment.
- **Keep the per-tier matrix small at first:** 3 scenarios × 4 models beats 30
  scenarios × 1 model for answering the "do weaker models degrade gracefully?"
  question early.
- Consider filing the resulting eval work as `wai` tickets so the research stays
  linked to implementation.
