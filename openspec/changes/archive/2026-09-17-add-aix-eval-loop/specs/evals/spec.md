# evals delta

## ADDED Requirements

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

Scenario reports SHALL optionally record which model replayed the scenario.

#### Scenario: report carries model when provided

- **WHEN** a replay is attributed to a model name
- **THEN** the scenario report SHALL include that name for matrix comparison

#### Scenario: report without attribution stays valid

- **WHEN** no model name is provided
- **THEN** the report SHALL serialize without a model field
