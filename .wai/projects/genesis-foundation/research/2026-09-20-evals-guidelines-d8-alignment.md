# D8 alignment: AI Evals FAQ (Hamel & Shankar, 2025-05-28) — adopted vs out of scope

Alignment record for `add-evals-guidelines` against
`program/evidence/evals-faq-hamel-shankar-2025-05-28.pdf` (stored in the
talleres program repo; sha256 b777f4b9…). Tier: METHOD guidance — never
proof, never a statistic.

## Adopted into the guidelines

1. **Manual error analysis before automation** — the guideline's scenario
   provenance requirement (every scenario traces to an observed failure
   source: corpus entry, error-analysis note, or ticket) encodes the FAQ's
   "look at your failures first" posture.
2. **Binary pass/fail over Likert** — deterministic checks assert pass/fail
   per check; no numeric scoring of agent behavior anywhere in the
   taxonomy or report.
3. **Prevalence bounding** — contrived-failure scenarios are declared as
   channel stress tests; their results bound *detection* claims, never
   prevalence claims.
4. **Staleness review** — a permanently green battery is never cited as
   evidence of quality; never-failing checks are reviewed and
   re-sharpened or retired (battery-maintenance requirement).
5. **Same-model judging caveats → avoided entirely** — genesis checks are
   deterministic predicates, so the FAQ's LLM-judge pitfalls don't apply;
   the guidelines forbid scoring free-form agent text (process-boundary
   scoring), which removes the judge-validation problem by construction.
6. **CI-set vs production-monitoring split** — mapped onto the tier ladder:
   tier 0/1 = CI gates, tier 2 = directional monitoring that never gates a
   push.

## Deliberately out of scope (and why)

1. **LLM-judge validation** (100–200 labeled examples per failure mode,
   TPR/TNR alignment) — genesis checks are deterministic; there is no
   judge to validate. If a consumer adds an LLM-judge check, the FAQ's
   method applies in *their* repo, at their tier-2 cadence.
2. **Production sampling / monitoring** — the guidelines govern pre-merge
   and scheduled batteries, not live production telemetry. The report
   contract could carry production rows later (additive evolution), but no
   requirement ships now.
3. **Error-analysis quantification targets** (≥30 traces, ≥100 to
   saturation, 60–80% dev time) — these are per-repo working practices,
   not suite-wide conventions; encoding them normatively would imply
   genesis audits consumer dev process, which the boundary rule forbids.
4. **Failure-mode dev/test split for judges** — same reason as 1: no
   judges in the suite-wide contract.
