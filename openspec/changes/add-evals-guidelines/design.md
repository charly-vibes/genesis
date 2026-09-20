# Design: evals guidelines

## D1 — Guidelines are a spec capability plus book docs, not a module

Two artifacts, one source of truth:

- **`evals-guidelines` spec capability** — the normative SHALLs. Changes
  to the rules go through openspec like any other behavior change, get
  reviewed, and are diffable. This is the source of truth.
- **mdBook pages** — the readable explanation (why weak readers, why the
  tier ladder, why raw attribution). The book explains; the spec governs.
  Cross-reference both ways: each book section cites the requirement it
  elaborates.

Alternative considered: prose-only docs (no spec). Rejected — prose
drifts silently; the consistency audits exist precisely because
unversioned conventions forked across repos.

## D2 — The live runner is per-consumer, deliberately

This change *defines* the action protocol and report contract but ships no
harness. Rationale:

- The harness is I/O-heavy, environment-specific (GHA vs local tick vs
  developer laptop), and per-tool repos differ in how they drive a model
  (pi, direct HTTP, recorded transcripts for CI).
- One shared implementation now would guess at five tools' deployment
  shapes. The boundary rule (`two or more tools need it`) triggers on
  demonstrated duplication, not prediction.
- Interoperability is preserved by contract, not by code: any harness
  following the action protocol and emitting the report JSON is a valid
  consumer.

Revisit trigger: when a second tool's harness shows the same shape as the
first's, extract the shared loop into `genesis::evals::replay`.

## D3 — Taxonomy stays shared code (the one thing that must not fork)

Fault codes are the join key for suite-wide dashboards and cross-tool
audits. If each repo invents its own strings, aggregation is guesswork.
So the one code artifact in this change is the new
`ERR_ACTION_FORMAT_VIOLATION` variant in `genesis::evals::ErrorTaxonomy`,
with round-trip tests guarding existing codes. Everything else in the
change is docs + spec.

## D4 — Report contract: versioned JSON, additive evolution

The report JSON carries a `report_version` (mirroring the envelope's
`envelope_version` pattern). Evolution rule: additive fields only within a
version; removing or re-typing a field bumps the version. Consumers
aggregate by `report_version`, so a tool running an older battery is
still legible. Required fields: scenario name, tool, outcome status
(`passed | failed | rate_limited | invalid_output`), per-check outcomes
(one per declared check, passing and failing) with an `ERR_*` code only
for agent faults and human-readable reasons (not parsed by
aggregators), and the bounds applied with their values. Required for
tier-2 rows, optional otherwise: model id (raw, verbatim, `:free`
suffix included) and repetition index (0-based, per scenario × model
cell). Optional: notes, fixture root reference. Optional fields are
omitted, never `null`. A bound-stopped trial ends `failed` with the
reached bound named. The three vocabularies stay distinct: status
(trial outcome), fault (agent-vs-tool attribution), `ERR_*` code
(shared taxonomy classification of an agent fault).

**Home of the contract in v1:** docs/fixture-normative — the reference
page (task 4.3) and the two checked-in fixtures (task 3.2) are the
source of truth. `ScenarioReport` (`src/evals.rs`) keeps its current
replay-tier shape until the first consumer harness lands; evolving it to
the contract then follows the additive rule (or a version bump), keeping
machinery out of genesis for now.

## D5 — Free tier semantics: entry tier, variability as signal

`:free` models are the default first candidates of the ordered registry —
chosen because they are the weakest readers of the output channels; a
channel that survives them is robust. Two consequences the guidelines make
normative:

- **Raw attribution**: ids recorded exactly as configured/reported
  (`deepseek/deepseek-v4-flash:free`), because `:free` deployments rotate
  and cross-time comparisons depend on the verbatim id.
- **No silent substitution**: on rate limiting, a trial ends
  `rate_limited` after one retry following a delay of at least two
  seconds (the delay used recorded in the report); model fallback is a
  caller-level decision that lands in the report. Registry ordering
  defines scheduling priority, not failover. Averaging/dedup belongs to
  the dashboard, not the battery.

## D6 — Tier ladder claims

Each tier may claim only what it measures, so evals can't be cited beyond
their evidence: tier 0 claims artifact presence/well-formedness; tier 1
claims check logic correctness over authored trajectories; tier 2 claims
agent-facing AIX behavior for the recorded model ids. A consumer citing
tier-1 results as "agents use the tool correctly" is violating the
guidelines — this is what the cadence column in the how-to encodes.

## D7 — Adoption path

Genesis delivers: spec + docs + taxonomy + report contract. Adoption is
per-repo tickets (dont, wai, espectacular, pretender, testaruda — vampiro
and successors after), each scoping its first tier-0 battery + one tier-1
scenario, deferring tier 2 until the battery is stable. Guidelines cite
the consumer's `openspec` change, keeping the loop closed.

## D8 — Alignment with external evals guidance

The guidelines adopt the practices from Hamel Husain & Shreya Shankar's
*AI Evals FAQ* (2025-05-28) that fit genesis's deterministic-only
stance, and deliberately diverge where the FAQ targets LLM-judge
centric stacks:

- **Error-analysis-first scenarios** (scenario provenance): the FAQ's
  central rule — evaluators come from observed failures, not imagined
  ones. Our taxonomy (`ErrorTaxonomy`) is already corpus-derived; the
  provenance requirement makes that discipline normative for consumer
  batteries. Contrived failures survive as an explicitly labeled
  category (channel stress tests) rather than being banned, because
  probing the tool's error channels is a stated genesis practice.
- **Prevalence bounding**: the FAQ warns synthetic data cannot measure
  real-world frequency. The tier-claims requirement gains an explicit
  scenario: contrived-failure results bound detection claims only.
- **Staleness review** (battery maintenance): the FAQ's "if everything
  keeps passing, the eval is no longer useful" becomes a normative
  review rule keyed to the tier-2 rotation cycle.
- **Binary over Likert, deterministic over judges, model-switch
  requires evidence**: already satisfied — binary checks, no LLM
  judges, fault routing that refuses to blame models without
  cross-model evidence.

Out of scope (FAQ topics that target production LLM products, not dev
tools): LLM-judge validation protocols (TPR/TNR alignment), production
trace sampling, and annotation-tooling guidance — those belong to
consumer repos if they ever adopt judges or production traffic.
