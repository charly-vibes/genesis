# Design: add-aix-eval-loop

## Context

Four modules touched (envelope, aix, evals, feedback) in one crate. The
constraint set comes from the corpus research plus two repo invariants:

- **Envelope stability:** downstream tools (wai, dont, espectacular, testaruda,
  vampiro, DDL) parse envelopes via `parse_envelope` and serde. The v0.6+
  caller-supplied `cli_version` contract was a coordinated 32-call-site
  migration — we will not repeat that cost for additive fields.
- **Boundary rule:** no domain logic in genesis. All four features are generic
  mechanisms; per-tool policies (which commands are mutating, what evidence
  means for a given tool) stay in the tools.

## Decisions

### D1 — Receipt metadata is additive and opt-in

`Envelope` gains `receipt: Option<ReceiptMeta>` with
`#[serde(skip_serializing_if = "Option::is_none")]`. Constructors keep their
current signatures; a builder method `with_receipt(ReceiptMeta)` opts in.

- `TerminalOutcome` enum: `Success | Failure | Timeout | Cancelled` (B173:
  "every external boundary needs an ending... silence cannot be neutral").
- `attempt: u32` + `idempotency_key: Option<String>` — retry visibility for
  mutating commands; tools decide which commands qualify.
- `evidence: Option<String>` — a verifiable statement of the user-visible edge
  ("file X exists at path Y"), not free-form narrative.

Rejected: separate `ReceiptEnvelope` type (fragmentation — two envelope shapes
means consumers must handle both); required fields (breaking).

### D2 — Token cost by heuristic, not tokenizer

Real tokenizers (tiktoken, hf-tokenizers) are heavyweight deps with model-bound
vocabularies — wrong for a crate distributed as a git dependency to six tools.
We use the chars/4 heuristic with a documented ±25% error band.

- `TokenCost { estimate: usize, heuristic: &'static str }` — the heuristic name
  ships with the number so the approximation is never mistaken for a count.
- **Compat:** existing `generate_llms_txt` / `generate_llm_txt` signatures are
  untouched (they return `String` and six call sites depend on them). The API
  adds (a) `estimate_token_cost(&str) -> TokenCost` as a free function over any
  generated artifact, and (b) bounded generation as *new* functions
  (`generate_llms_txt_bounded`, `generate_llm_txt_bounded`) taking a max-token
  budget. Budget degradation order (deterministic, testable):
  1. truncate module descriptions to first sentence
  2. drop optional sections (`LlmSection::raw` marked optional)
  3. drop per-module tables, keep headings
  Both generators are budget-addressable (modules-based llms.txt and
  section-based llm.txt). Budget degradation mirrors B128's "reducing slowly
  the amount of description" rather than dropping whole artifacts.

Rejected: pulling a tokenizer (dep weight, model coupling); refusing to
generate over budget (silent missing artifacts are worse than verbose ones).

### D3 — Distractors as fixture files, doc-drift as a composable check

`Scenario` already replays `Vec<AgentStep>` against fixture files. Distractors
are just fixture files with declared *stale* metadata:

- `distractor_file(path, content, DistractorKind::StaleDocs)` — participates in
  the replay environment; the check decides whether consulting it is a fault.
- `doc_drift_blindness()` — passes iff the agent invoked the tool and acted on
  the envelope despite a distractor contradicting it. Composes with existing
  checks (ok_envelope, agent_followed_hint) rather than replacing them.
- **Threading:** checks receive `&ScenarioResult`, so `ScenarioResult` must
  carry the distractor registry (path + kind + content) populated by `run()`;
  the check reads it from there rather than from the scenario.

The model that replayed a scenario is recorded on `ScenarioReport`
(`model: Option<String>`) to support per-model matrix runs without changing
`run()`'s signature — matrix orchestration stays outside the crate.

### D4 — Feedback converts to Scenarios; fixtures are caller-supplied

The existing substrate constrains what a conversion can read:
`ContextBundle` (feedback/context.rs) carries command, exit_code,
suggestion_footer, and environment state — **no file snapshots**; `ErrorRecord`
(feedback/scratch.rs) carries argv, exit, footer, kind — no captured output.

Therefore `Scenario::from_feedback_context(bundle, fixtures)` takes the bundle
plus an explicit `Vec<(path, content)>` fixture list supplied by the caller
(the converting tool knows which files were in play; genesis must not
re-snapshot the working tree):

- Bundle-only conversion (`fixtures` empty) yields a prompt-only scenario built
  from command + expected error envelope with the captured hint — replayable
  against recorded steps, no LLM or subprocess.
- With fixtures, the files become scenario fixtures via the existing
  `fixture_file()` mechanism.
- One-way (Scenario → feedback bundle is meaningless); no shared mutation.
- A convenience `from_last_error(tool_name, fixtures)` wraps
  `scratch::read_last_error` for the `--from-last-error` path.

## Risks

- **Heuristic drift:** if chars/4 proves too loose for the AIX-ablation metric,
  revisit with a pluggable `TokenEstimator` trait (not built now — YAGNI until
  the ablation produces data).
- **Receipt under-adoption:** tools may never set `receipt`. Mitigation: the
  evals module's checks can assert receipt presence for mutating commands,
  making adoption visible in each tool's own eval suite.
- **Conversion fidelity:** bundle-only scenarios (no fixtures) are weak
  regression tests. Mitigation: feedback report guidance steers reporters
  toward attaching fixture context; the gh issue body template already gathers
  environment state that hints at fixture candidates.
