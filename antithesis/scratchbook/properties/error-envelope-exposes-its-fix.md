# error-envelope-exposes-its-fix — evidence

Extends the suite-wide entry in
`charly/antithesis/scratchbook/properties/genesis.md` (2026-11-16 scan, crate
commit `a6a5150`). This file reflects genesis `ddf412d`.

## What led to this property

The 2026-11-16 scan found the reader (`parse_envelope`, then
`src/evals/outcome.rs`) consumed only the legacy top-level `hints` array,
while `Envelope::error` set `hints: None` and put fixes solely in
`data.remediation[]` — a named fix was invisible to the crate's own consumer
helper. The prior run's open question asked which channel is canonical.

## Resolution of the prior finding (code changed since)

At `ddf412d` the reader lives at `src/envelope/outcome.rs` and unions both
channels (`hint_commands`, `outcome.rs:155-172`): error-channel commands
first, then legacy hints, counting `malformed_hints` for entries naming no
command. `Envelope::error` still sets `hints: None` (`envelope.rs:289`) but
that no longer hides remediation. The prior RED is **GREEN** at this commit;
the constructor→reader direction of the property holds.

## Remaining defect: Invariant 3.2.5 bypass on ingress

`ErrorResult::new` rejects empty remediation (`envelope.rs:140-160`,
Invariant 3.2.5, doc-comment enforced since the dont port). But `ErrorResult`
derives `Deserialize` with no custom validation, so:

```json
{"ok": false, ..., "data": {"code": "E_X", "message": "m", "rule_name": null,
 "spec_ref": null, "entity_id": null, "unmet_clauses": [], "remediation": []},
 "warnings": [], "hints": null, "meta": {...}}
```

deserializes successfully into an `Envelope<ErrorResult>` that the type's own
invariant says cannot exist. Verified empirically (scratch test
`p1_deserialized_error_envelope_invariant`).

Incidental finding from the first (failed) attempt at that test:
`spec_ref`, `entity_id`, and `unmet_clauses` have **no `#[serde(default)]`**,
so JSON missing those fields is rejected outright. That strictness is
inconsistent with the lenient-reader philosophy of `parse_envelope_value`
("only `ok` is required … holds across envelope-version drift"): the type
deserialize is stricter than the value reader, and an older producer omitting
a newer field fails type-deserialization while `parse_envelope` would have
accepted the same payload.

## Execution record (scratch crate `antithesis/scratch-hegel`)

| Test | Outcome |
|---|---|
| `p1_error_envelope_remediation_visible` (constructor → JSON → `parse_envelope_value` exposes `genesis doctor --fix`) | GREEN |
| `p1_deserialized_error_envelope_invariant` (empty remediation rejected on deserialize) | RED — bypass confirmed |
| `p1_error_envelope_with_no_fix_anywhere_is_visible` (reader reports zero commands faithfully, no phantoms) | GREEN |

## Consumer version skew

A fix (custom `Deserialize`, serde field defaults, or doc-level
acknowledgment) ships in the next `genesis-vibes` release:

- **Reaches on rebuild** (requirement `^0.14`, locked 0.14.0): `dont`
  (`=0.14.0` req), `wai`, `testaruda`, `evallerina`, `dulce-de-leche`.
- **Does not reach** without an explicit requirement bump: `whisper`
  (`^0.13.1`, locked 0.13.1), `gently-cli` (`^0.13.2`), `specodelic`
  (`^0.13`, locked 0.13.2), `specodelic-gre-wt` (`^0.11`, locked 0.11.1),
  `bajan` (`^0.8`, locked 0.8.3).

The evals machinery that *scores* tools via `parse_envelope` is often vendored
or versioned per-consumer; a consumer pinned below the fix keeps its current
envelope behavior in both directions (neither the bypass fix nor the
two-channel reader if pinned below the release that added it).

## Open Questions

- Does any consumer deserialize into `Envelope<ErrorResult>` from untrusted
  bytes (making the ingress bypass reachable in practice), or do all
  consumers only construct envelopes in-process? The four `^0.14` consumers'
  call sites were not exhaustively enumerated.

## Investigation Log

#### Is `data.remediation` canonical and `hints` secondary, or is the constructor wrong?

- Examined: `src/envelope.rs` (constructors, Invariant 3.2.5),
  `src/envelope/outcome.rs` (two-channel union), `src/envelope/notice.rs`,
  prior golden files under `src/envelope/tests/`.
- Found: the question is answered in code — the reader treats
  `data.remediation[]` as primary (searched first) and `hints` as legacy
  secondary; `Envelope::error`'s `hints: None` is now harmless. The prior
  RED is fixed; the remaining gap moved to the deserialize path.
- Conclusion: resolved; property restated so it holds under either fix
  direction ("a named fix is visible to consumers").
