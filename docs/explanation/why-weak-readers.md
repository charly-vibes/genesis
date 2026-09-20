# Why Weak Readers

Why the eval tier ladder makes OpenRouter `:free` model ids the **entry tier**
of the model registry, and why variability across weak models is a signal, not
noise.

*(Elaborates the **Free-tier entry and attribution** requirement of the
`evals-guidelines` spec — `openspec/specs/evals-guidelines/spec.md`.)*

## The weakest reader defines the channel's strength

A tool communicates with agents through structured channels: the JSON
envelope, hints, self-healing suggestions, managed blocks, discovery
artifacts. Frontier models will often recover from a muddy signal by
brute-forcing — reading your source, guessing flags, retrying. That recovery
**hides** channel defects: the model succeeds despite the channel, and the
battery goes green while real users' weaker models fail.

The `:free` tier inverts this. A weak model that cannot compensate has one
path to success: parse the envelope, read the hint, follow it. If a weak
model parses the envelope and honors the hint, the channel is robust — the
protocol, not the model, carried the interaction. That is the property worth
measuring.

## Variability as signal

Weak models are noisy: the same scenario yields different trajectories run
to run. In this design the variance is not an annoyance to average away — it
is the measurement:

- **High pass-rate variance across repetitions** on one scenario says the
  scenario's signal is marginal relative to the channel's noise floor —
  the scenario is not yet sharp enough to discriminate.
- **A failure that reproduces across ≥ 3 weak model ids** (or all configured
  ids when fewer) points at the output channel, not the model — route it to
  AIX work in genesis, not to a model-choice decision.
- **A failure isolated to one model id** stays directional evidence; it does
  not route anywhere by itself.

This is why the contract records the raw model id **verbatim** (including
`:free`) and one row per repetition: matrix comparisons across time depend on
unnormalized ids and distinct rows. Aggregated averages would erase exactly
the variance that is the signal.

## Nothing special-cased

The ordering is the only privilege `:free` ids get: they are the default
first candidates of an ordered registry, which defines trial scheduling
priority. No failover semantics, no scoring bonus, no separate code path —
model fallback is a caller-level decision, recorded per trial when used. A
weak reader that passes is a channel success; a frontier model that fails is
a channel finding too, and often a cheaper one to reproduce on the entry
tier first.

## Trade-offs, stated plainly

- Free tiers are rate-limited (hence the 429 protocol: one spaced retry,
  then `rate_limited` — recorded, never silently substituted) and can be
  slow or de-prioritized at peak; tier-2 runs are scheduled, not gating.
- Weak readers cap scenario complexity: intents a weak model cannot parse at
  all measure the model, not the channel. Scale prompt specificity by tier
  (exact subcommands for entry tiers, intent-only for frontier tiers) so
  each tier measures the channel within its reach.
- Free-tier ids rotate and deprecate; the verbatim id + report rows make the
  drift visible instead of silently changing what "the battery" measured.

## Related

- [Creating Agent Evals](../how-to/evals.md) — the tier ladder and action protocol
- [Running the ladder in CI](../how-to/evals-ci.md) — rotation, 429 semantics, budget
- [The eval report contract](../reference/eval-report.md) — raw attribution, repetition rows
