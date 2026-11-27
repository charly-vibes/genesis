# evals Specification

## Purpose
TBD - created by archiving change add-aix-eval-loop. Update Purpose after archive.
## Requirements
### Requirement: Distractor fixtures in scenarios

genesis SHALL allow scenarios to declare distractor files that exist in the
replay environment but are not part of the task's happy path.

#### Scenario: distractor is visible to the replay environment

- **WHEN** a scenario declares a distractor file and runs against agent steps
- **THEN** the fixture environment SHALL contain the distractor file at its
  declared path

#### Scenario: distractors do not fail an otherwise-correct run

- **WHEN** the agent ignores a distractor and satisfies all scenario checks
- **THEN** the scenario report SHALL pass

### Requirement: Doc-drift blindness check

genesis SHALL provide a reusable check asserting that the agent trusted the
tool's structured output over stale documentation contradicting it.

#### Scenario: agent trusts envelope over stale docs

- **WHEN** a scenario includes a stale-docs distractor contradicting the tool's
  envelope
- **AND** the agent's steps show tool invocation followed by envelope-consistent
  action
- **THEN** the `doc_drift_blindness` check SHALL pass

#### Scenario: agent follows stale docs over the envelope

- **WHEN** the agent's steps show action consistent with the stale doc and
  inconsistent with the envelope
- **THEN** the check SHALL fail as `agent_fault`
- **AND** the reason SHALL identify the distractor path

### Requirement: Model attribution on scenario reports

Scenario reports SHALL optionally record which model replayed the scenario,
recording the raw model id verbatim — including `:free` and any deployment
suffix — because matrix comparisons across time depend on the unnormalized
id. (Repetition indexes, added by the evals-guidelines capability, ride on
the same report.)

#### Scenario: report carries model when provided

- **WHEN** a replay is attributed to a model name
- **THEN** the scenario report SHALL include that name for matrix
  comparison
- **AND** the id SHALL be recorded verbatim, including `:free` or
  deployment suffixes

#### Scenario: report without attribution stays valid

- **WHEN** no model name is provided
- **THEN** the report SHALL serialize without a model field

### Requirement: Action-format-violation fault code

genesis SHALL extend `ErrorTaxonomy` with `ActionFormatViolation`
(`ERR_ACTION_FORMAT_VIOLATION`): the live agent failed to produce a
parseable structured action after one re-ask. It SHALL be distinct from
`ToolExecutionHallucination`, which remains a replay-tier code for claims
of runs that never happened.

#### Scenario: round-trip through the code

- **WHEN** `from_code("ERR_ACTION_FORMAT_VIOLATION")` is called
- **THEN** it SHALL return the `ActionFormatViolation` variant
- **AND** `code()` on that variant SHALL return the same string

#### Scenario: existing codes unchanged

- **WHEN** any pre-existing `ERR_*` code is round-tripped
- **THEN** the mapping SHALL be unchanged

### Requirement: Receipt terminal-outcome check

A composable `ScenarioCheck` SHALL exist that evaluates a run's final
envelope: it passes when the envelope carries `receipt.terminal_outcome`.
The check itself does not know which runs are mutating — the caller scopes
it (e.g., applies it only to steps it has marked as mutating), mirroring
the envelope contract where tools decide which commands qualify as
mutating.

#### Scenario: envelope with receipt passes the check

- **WHEN** the replayed step's final envelope contains a receipt with a
  terminal outcome
- **THEN** the check SHALL return a passing `CheckOutcome`

#### Scenario: missing receipt is a tool fault

- **WHEN** the caller applies the check to a step it has marked as mutating
  and the replayed step's final envelope carries no receipt
- **THEN** the check SHALL return `tool_fault` citing the missing receipt

#### Scenario: success receipt over failing envelope is a lie

- **WHEN** the final envelope has `ok: false` but its receipt records
  `terminal_outcome: success`
- **THEN** the check SHALL return `tool_fault` with a reason naming the
  contradiction


### Requirement: Dual-channel hint resolution

`parse_envelope` SHALL resolve the hint commands of an error envelope from
both the `data.remediation` channel and the top-level `hints` channel, and
SHALL return the union in a stable order (error channel first). A hint entry
MAY be an object carrying `command`, or a bare string naming a command.

#### Scenario: remediation in the error channel is resolved

- **WHEN** `parse_envelope` reads an error envelope whose `data.remediation`
  names one command
- **THEN** the returned outcome SHALL contain that command among its hint
  commands

#### Scenario: top-level hints are resolved

- **WHEN** `parse_envelope` reads an error envelope whose top-level `hints`
  array names one command
- **THEN** the returned outcome SHALL contain that command among its hint
  commands

#### Scenario: both channels contribute

- **WHEN** an error envelope carries a command in `data.remediation` and
  another in the top-level `hints`
- **THEN** the returned outcome SHALL contain both commands
- **AND** the error-channel command SHALL come first

#### Scenario: string-valued hint entries are tolerated

- **WHEN** a hint entry is a bare string naming a command
- **THEN** `parse_envelope` SHALL treat it as that command
- **AND** SHALL NOT fail

#### Scenario: conflicting channels keep both commands

- **WHEN** an error envelope names one command in `data.remediation` and a
      *different* command in the top-level `hints`
- **THEN** the returned outcome SHALL contain both commands
- **AND** SHALL NOT discard either channel's command

#### Scenario: an envelope with no hint channel resolves to no commands

- **WHEN** an error envelope carries neither remediation nor hints
- **THEN** the returned outcome SHALL contain zero hint commands
- **AND** SHALL NOT report an error

### Requirement: Eval fixtures use genesis-built envelopes

Scenario fixtures that stand in for a tool's output SHALL be produced by the
genesis envelope constructors, not hand-written JSON literals. A fixture built
with `Envelope::error` and a remediation entry SHALL resolve to a non-empty
hint-command list.

#### Scenario: a genesis-built error envelope resolves its remediation

- **WHEN** a fixture is built with `Envelope::error` carrying an
  `ErrorResult` with one remediation entry
- **THEN** `parse_envelope` SHALL resolve that entry's command

#### Scenario: a genesis-built success envelope parses as ok

- **WHEN** a fixture is built with `Envelope::success`
- **THEN** `parse_envelope` SHALL report the `Ok` outcome

### Requirement: Hint-following checks are validated against the real shape

The hint-following checks SHALL be exercised against envelopes produced by the
genesis envelope constructors, so that a divergence between the writer shape
and the reader shape fails the suite.

#### Scenario: hint-following scenario passes against a real error envelope

- **WHEN** the hint-following scenario replays a step whose stdout is a
  genesis-built error envelope naming a fix command
- **THEN** the hint check SHALL pass
- **AND** the report SHALL record no failure for that check

#### Scenario: a writer/reader divergence fails the suite

- **WHEN** the envelope constructors stop emitting a channel that
  `parse_envelope` reads
- **THEN** the hint-following scenario SHALL fail
