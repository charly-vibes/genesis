# evals-guidelines Specification

## Purpose
TBD - created by archiving change add-evals-guidelines. Update Purpose after archive.
## Requirements
### Requirement: Tier ladder

Consumer eval batteries SHALL be organized as three tiers with distinct
cadences and claims: tier 0 static checks (no model, every push), tier 1
scripted replay of authored trajectories (no model, nightly), tier 2 live
model runs (scheduled, rotated across model × scenario cells). A tier's
results SHALL NOT be cited beyond what that tier measures.

#### Scenario: tier claims are bounded

- **WHEN** a consumer documents its eval results
- **THEN** tier-1 results SHALL be presented as check-logic validation
  over authored trajectories, not as evidence of live agent behavior

#### Scenario: tier cadences

- **WHEN** a consumer configures CI
- **THEN** tier 0 SHALL run on every push, tier 1 on a nightly schedule,
  and tier 2 on a scheduled rotation that runs every configured cell
  (scenario × model id) at least once per declared cycle, with a cycle
  no longer than 14 days
- **AND** the rotation schedule SHALL be declared in the consumer's CI
  configuration

#### Scenario: contrived failures bound prevalence claims

- **WHEN** results come from a scenario whose failures are injected or
  contrived
- **THEN** they SHALL be presented as detection-capability evidence for
  the tool's error channels, not as an estimate of real-world failure
  frequency

### Requirement: Battery maintenance

Consumers SHALL review the battery for staleness: a check that has
never failed since its scenario was created SHALL be reviewed for
continued relevance (kept, sharpened, or retired) before the next
tier-2 rotation cycle, and the decision SHALL be recorded in the
scenario's source. A permanently green battery SHALL NOT be cited as
evidence of tool quality.

#### Scenario: never-failing checks are reviewed

- **WHEN** a check has never failed across its scenario's recorded runs
- **THEN** the consumer SHALL review it for continued relevance before
  the next tier-2 rotation cycle
- **AND** the decision SHALL be recorded in the scenario's source

### Requirement: Process-boundary scoring

Checks SHALL assert only on signals that crossed the process boundary:
subprocess exit codes, captured stdout/stderr envelopes, and filesystem
state. Free-form agent text SHALL NOT be scored.

#### Scenario: envelope is the assertion target

- **WHEN** a check validates a tool's error handling
- **THEN** it SHALL parse the captured stdout envelope and assert on
  `ok`, error codes, and hint commands — not on agent commentary

#### Scenario: hallucination remains detectable

- **WHEN** an agent claims a command ran without a corresponding
  subprocess spawn
- **THEN** the step's `executed` field SHALL be false
- **AND** the scenario MAY classify this as
  `ERR_TOOL_EXECUTION_HALLUCINATION`

### Requirement: Fixture provisioning

Each scenario SHALL declare a fixture (tool binary pre-installed on
`PATH` before the agent turn) and an agent prompt.

#### Scenario: tool binary provisioned before the turn

- **WHEN** a scenario is provisioned
- **THEN** the tool binary SHALL be pre-built and on the sandbox `PATH`
- **AND** compiling from source inside a trial SHALL NOT occur unless the
  scenario is explicitly about installation

### Requirement: Distractor registration

Scenarios MAY register distractor files, which SHALL be registered
separately from task material. Contrived failures, where applicable,
SHALL be declared as part of the fixture declaration.

#### Scenario: distractors are not task material

- **WHEN** a scenario includes distractor files
- **THEN** they SHALL live in the distractor registry, distinct from the
  fixture's task material

### Requirement: Scenario provenance

Every scenario SHALL trace to an observed failure source recorded in
its declaration: an entry in the tool's failure corpus, an
error-analysis note, or a ticket. Contrived failures MAY stress-test an
error channel, but such scenarios SHALL be declared as channel stress
tests — distinct from observed-failure scenarios — and their results
SHALL NOT be cited as evidence about failure modes they do not probe.

#### Scenario: provenance is recorded

- **WHEN** a scenario is authored
- **THEN** its declaration SHALL name the observed failure source it
  derives from, or declare itself a channel stress test

### Requirement: One fault per check

Checks SHALL be deterministic, and each check SHALL attribute at most
one fault: either a tool fault (no taxonomy code) or exactly one
agent-fault taxonomy code.

#### Scenario: one fault per check

- **WHEN** a check fails
- **THEN** it SHALL attribute either a tool fault (no taxonomy code) or
  exactly one agent-fault taxonomy code

### Requirement: Empty-trajectory guard

Every tier-2 scenario SHALL include at least one empty-trajectory guard:
a check that fails when the recorded trajectory contains zero executed
steps.

#### Scenario: passive models cannot pass by doing nothing

- **WHEN** a live model returns an immediate done signal with zero
  executed steps
- **THEN** the scenario's empty-trajectory guard SHALL fail
- **AND** the trial SHALL NOT be recorded as passed

### Requirement: Live action protocol

Consumer harnesses running tier 2 SHALL require the agent to return one
structured action per turn (exact command line plus done flag). Each turn
the model SHALL receive a standardized minimum context: the scenario
prompt and the fixture root path inside the sandbox. The **fixture
surface** — the files the fixture contains and their contents — is
observed only through executing actions. The context SHALL never include
the check logic, the expected outcomes, or the distractor registry. On
malformed output the harness SHALL re-ask once with the format error
attached (the re-ask consumes a turn of the cap); persistent malformation
SHALL end the trial as `invalid_output` with agent fault
`ERR_ACTION_FORMAT_VIOLATION`. Every live trial SHALL be bounded by a
turn cap, a wall-clock timeout, and a token-estimate budget — measured
with the `aix` chars/4 heuristic over the trial's full prompt and
completion traffic, and labeled as heuristic. The applied cap, timeout,
and budget values SHALL be recorded in the report. A trial stopped at a
bound SHALL end `failed`, SHALL name the reached bound in the report,
and the bound stop itself SHALL NOT be attributed as an agent or tool
fault.

#### Scenario: one re-ask on malformed output

- **WHEN** the model's response is not a parseable action
- **THEN** the harness SHALL re-ask once, including the format error

#### Scenario: re-asks consume the turn cap

- **WHEN** a malformed response triggers the one re-ask
- **THEN** that re-ask SHALL count as one turn toward the cap

#### Scenario: persistent malformation is an agent fault

- **WHEN** the retried response is also not a parseable action
- **THEN** the trial SHALL end `invalid_output`
- **AND** the report SHALL carry `ERR_ACTION_FORMAT_VIOLATION` as an
  agent fault, distinct from `ERR_TOOL_EXECUTION_HALLUCINATION`

#### Scenario: parseable actions are executed once

- **WHEN** the model returns a parseable action
- **THEN** the harness SHALL execute the command exactly once
- **AND** the resulting step SHALL be recorded with `executed: true`

#### Scenario: bounds stop runaway trials

- **WHEN** a trial exceeds its turn cap, wall-clock timeout, or token
  budget
- **THEN** the harness SHALL stop at the first bound reached
- **AND** the trial SHALL end `failed` with the reached bound named in
  the report and no agent- or tool-fault attribution for the stop itself

#### Scenario: check logic is never exposed

- **WHEN** the harness builds the model's per-turn context
- **THEN** it SHALL include only the scenario prompt and the fixture
  root path inside the sandbox
- **AND** it SHALL NOT include check definitions, expected outcomes, or
  the distractor registry

### Requirement: Free-tier entry and attribution

The model registry SHALL be ordered with OpenRouter `:free` model ids as
the default first candidates of the registry — nothing about them is
special-cased beyond that ordering. The ordering defines trial
scheduling priority (which cells run first); it is not failover
semantics — model fallback is a caller-level decision that SHALL be
recorded when used. Reports SHALL record the raw model id verbatim
(including the `:free` suffix), one row per repetition, indexed 0-based
per scenario × model cell. On HTTP 429 the harness SHALL retry once
after a delay of at least two seconds; the delay used SHALL be recorded
in the report. A second 429 SHALL end the trial `rate_limited` — never
attributed as a tool fault, never silently retried against a different
model.

#### Scenario: raw attribution

- **WHEN** a trial runs against `deepseek/deepseek-v4-flash:free`
- **THEN** the report SHALL record that id verbatim

#### Scenario: repetitions are distinct rows

- **WHEN** a cell (scenario × model) is replayed more than once
- **THEN** each repetition SHALL be a separate report row

#### Scenario: 429 is retried once then recorded

- **WHEN** two consecutive OpenRouter calls return 429
- **THEN** the trial SHALL end `rate_limited` with no fault attribution

#### Scenario: no silent model substitution

- **WHEN** a harness falls back to a different model for a cell
- **THEN** the substitution SHALL be recorded
- **AND** each trial's report SHALL name the model that actually produced
  its trajectory

### Requirement: Sandbox confinement

Tier-2 execution SHALL run in a sandbox with isolated `HOME`, scrubbed
environment, network egress denied unless the scenario opts in, and a
per-command timeout that terminates runaway processes. The harness SHALL
execute each action exactly once and record captured streams verbatim.

#### Scenario: isolated execution

- **WHEN** a live action is executed
- **THEN** it SHALL run with the sandbox's isolated `HOME` and environment
- **AND** the resulting step's `executed` field SHALL be true

#### Scenario: network denied by default

- **WHEN** a live action attempts network access and the scenario
  declaration does not request network access
- **THEN** the attempt SHALL fail within the sandbox

#### Scenario: runaway command terminated

- **WHEN** an action exceeds the per-command timeout
- **THEN** the process SHALL be terminated and the timeout recorded
- **AND** the terminated step SHALL be recorded as a distinct step
  outcome
- **AND** checks MAY treat post-termination filesystem state as
  indeterminate rather than attributing it to the agent

### Requirement: Interoperable report contract

Scenario reports SHALL serialize to a versioned JSON shape
(`report_version`, additive evolution only within a version) carrying at
minimum: scenario name, tool, outcome status (`passed | failed |
rate_limited | invalid_output`), per-check outcomes — one per declared
check, passing and failing — each carrying an outcome status, an
`ERR_*` code when and only when the failure is an agent fault, and a
human-readable reason (for humans and tickets, not parsed by
aggregators), and the bounds applied with their values. Optional fields
SHALL be omitted, never `null`. Tier-2 rows SHALL additionally carry the
raw model id and repetition index (0-based, per scenario × model cell);
non-tier-2 rows MAY omit them. Three vocabularies are distinct and defined as follows: the
**status** is the trial outcome (`invalid_output` carries the
`ERR_ACTION_FORMAT_VIOLATION` agent fault; `rate_limited` carries no
fault); a **fault** is the agent-vs-tool attribution of a failure; an
**`ERR_*` code** is the shared taxonomy classification of an agent
fault. The contract SHALL be published as a reference page with an
example.

#### Scenario: tool faults carry no code

- **WHEN** a check fails with a tool fault
- **THEN** its per-check outcome SHALL carry a reason but no `ERR_*`
  code
- **AND** the code field SHALL be omitted, not `null`

#### Scenario: suite-wide aggregation

- **WHEN** two tools emit reports under the same `report_version`
- **THEN** a dashboard SHALL aggregate them without per-tool mapping
- **AND** rows without model attribution SHALL group as non-tier-2

#### Scenario: additive evolution

- **WHEN** a new optional field is added to the report
- **THEN** `report_version` SHALL remain unchanged
- **AND** older consumers SHALL ignore the unknown field

#### Scenario: breaking change bumps the version

- **WHEN** a field is removed or re-typed
- **THEN** `report_version` SHALL be incremented

### Requirement: Fault routing

Tool faults SHALL be filed as tickets in the tool's repository and SHALL
NOT be routed as AIX-channel work. Agent faults SHALL be routed by
grouping the `ERR_*` agent-fault code within the same output channel
(the suggestions, envelope, managed-block, or artifact surface the check
probes): when the same agent-fault code appears across three or more
distinct model ids for that channel — or across **all** configured ids
when fewer than three are configured — the finding SHALL be routed as
AIX-channel work (work on the output channel itself, e.g. a hint channel
that weak models reliably miss → suggestions-module work in genesis or
the tool's AIX surface), not dismissed as model noise. Agent faults
below the routing threshold for their channel — one model id, or fewer
than the threshold within the channel — SHALL be recorded without
action. The scenario is a reporting dimension, not the routing key.

#### Scenario: tool fault becomes a ticket

- **WHEN** a check fails with a tool fault
- **THEN** a ticket SHALL be filed in the tool's tracker referencing the
  report

#### Scenario: cross-model agent fault points at the channel

- **WHEN** the same agent-fault code appears across the routing
  threshold of model ids for the same output channel
- **THEN** the finding SHALL be routed to AIX-channel work, not to the
  model
- **AND** single-model occurrences SHALL be recorded without action

#### Scenario: sub-threshold occurrences are recorded without action

- **WHEN** the same agent-fault code appears in fewer model ids than the
  routing threshold for an output channel (e.g. two of four configured)
- **THEN** the finding SHALL be recorded without routing action

#### Scenario: small registries can still route

- **WHEN** a consumer configures fewer than three model ids
- **AND** the same agent-fault code appears across all of them for the
  same output channel
- **THEN** the finding SHALL be routed to AIX-channel work

