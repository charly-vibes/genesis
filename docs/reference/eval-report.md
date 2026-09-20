# Eval Report Contract

The interoperable JSON contract for scenario eval reports
(`add-evals-guidelines`, requirement *Interoperable report contract*). A
report describes one trial: one scenario, replayed or run live, in one
tool repo. Dashboards aggregate rows by `report_version` with no per-tool
mapping.

The contract is **normative at the docs/fixture level**: this page and the
two checked-in fixtures are the source of truth for v1.

- Tier-2 example (live trial, failed): `tests/golden/eval_report_tier2.json`
- Non-tier-2 example (replay trial, passed): `tests/golden/eval_report_replay.json`

## Contract (report_version 1)

### Required fields

| Field | Type | Meaning |
|---|---|---|
| `report_version` | integer | Contract version; `1` for this shape. |
| `name` | string | Scenario name. |
| `tool` | string | The tool repo that produced the row. |
| `status` | string | Trial outcome: `passed`, `failed`, `rate_limited`, or `invalid_output`. |
| `checks` | array | One entry per **declared** check — passing and failing (never only the failures). |
| `bounds` | object | The bounds applied with their values. Empty object for tiers that apply no bounds — present, never `null`. |

### Per-check entries

Each element of `checks` carries:

| Field | Type | Meaning |
|---|---|---|
| `name` | string | Stable check name. |
| `status` | string | `passed` or `failed`. |
| `fault` | string | Present only when `status` is `failed`: `agent` or `tool`. |
| `code` | string | Present **only when** the failure is an agent fault. An `ERR_*` taxonomy code. Omitted — never `null` — for tool faults and passes. |
| `reason` | string | Human-readable, agent-actionable reason — for humans and tickets, not parsed by aggregators. |

### Tier-2 fields (required for live rows, optional otherwise)

| Field | Type | Meaning |
|---|---|---|
| `model` | string | Raw model id **verbatim**, including `:free` and any deployment suffix. |
| `repetition` | integer | 0-based, per scenario × model cell. Each repetition is a distinct row. |

Rows without attribution group as non-tier-2 in aggregation.

### Optional fields (omit, never `null`)

| Field | Type | Meaning |
|---|---|---|
| `bound_reached` | string | Name of the bound that stopped the trial (`max_turns`, `timeout_secs`, `token_budget`). A bound-stopped trial ends `failed`; the stop itself carries **no** agent or tool fault. |
| `retry_delay_secs` | number | The 429 retry delay actually used (at least 2 seconds before the single retry). |
| `notes` | string | Free-form notes for humans. |
| `fixture_root` | string | Reference to the fixture root, when useful for reproduction. |

### The three vocabularies are distinct

1. **`status`** — the trial outcome. `invalid_output` carries the
   `ERR_ACTION_FORMAT_VIOLATION` agent fault; `rate_limited` carries no
   fault.
2. **`fault`** — the agent-vs-tool attribution of a failed check (or an
   `invalid_output` trial).
3. **`code`** — the shared `ERR_*` taxonomy classification of an agent
   fault. Never present on a tool fault.

### Evolution rules

- **Additive** — a new optional field keeps `report_version` at `1`; older
  consumers ignore unknown fields.
- **Breaking** — removing or re-typing a field bumps `report_version`.
  Consumers aggregate by version; a tool running an older battery stays
  legible.

## Delta against the current `ScenarioReport`

`ScenarioReport` (`src/evals.rs`) keeps its replay-tier shape until the
first consumer harness lands (design D2/D4: no live-runner machinery in
genesis). Mapping and delta:

| Contract v1 | `ScenarioReport` today | Delta |
|---|---|---|
| `name` | `name` | none |
| `status` | `passed: bool` | string enum vs boolean; `passed` ⇔ `status == "passed"` |
| `checks` (all declared, passing and failing) | `failures` (failures only) | replay rows currently list failures only; contract requires every declared check |
| `code` | `taxonomy_code` per failure | same vocabulary, different field name; omitted for tool faults in both |
| `tool` | — | live-only; not serialized today |
| `bounds`, `bound_reached` | — | live-only; not serialized today |
| `model` | `model` | same; omitted when absent in both |
| `repetition` | — | live-only; not serialized today |
| `report_version` | — | live-only; not serialized today |

`ScenarioReport`'s serialization is validated against the non-tier-2
fixture by `tests/eval_report_contract.rs`. When a consumer harness
lands, `ScenarioReport` evolves to this shape additively (or the version
bumps) — per the additive-evolution rule.
