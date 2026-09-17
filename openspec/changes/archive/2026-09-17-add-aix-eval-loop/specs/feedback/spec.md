# feedback delta

## ADDED Requirements

### Requirement: Feedback context converts to scenarios

genesis SHALL convert captured feedback context (a ContextBundle, optionally
with caller-supplied fixture files) into a replayable Scenario so that
agent-reported failures become regression scenarios.

#### Scenario: bundle-only conversion yields a prompt-only scenario

- **WHEN** `from_feedback_context` is called with a ContextBundle and no
  fixture files
- **THEN** conversion SHALL succeed with a prompt-only Scenario derived from
  the recorded command and error expectation
- **AND** the Scenario SHALL expect an error envelope carrying the bundle's
  suggestion footer as the hint

#### Scenario: caller-supplied fixtures become scenario fixtures

- **WHEN** `from_feedback_context` is called with a bundle and a fixture list
  of (path, content) pairs
- **THEN** the converted Scenario SHALL contain those fixtures via the
  existing fixture mechanism

#### Scenario: last-error path converts from scratch

- **WHEN** `from_last_error(tool_name, fixtures)` is called and a scratch
  ErrorRecord exists
- **THEN** the converted Scenario SHALL reflect the recorded argv, exit code,
  and footer

#### Scenario: missing scratch record is an error, not a panic

- **WHEN** `from_last_error` is called with no scratch record for the tool
- **THEN** conversion SHALL return a typed error

#### Scenario: converted scenario runs without a live agent

- **WHEN** the converted scenario is replayed against recorded AgentStep
  transcripts
- **THEN** no subprocess runner or LLM SHALL be required
