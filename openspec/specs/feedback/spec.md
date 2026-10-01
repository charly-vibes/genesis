# feedback Specification

## Purpose
TBD - created by archiving change add-aix-eval-loop. Update Purpose after archive.
## Requirements
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

### Requirement: aix-gap feedback kind

The feedback flow SHALL accept `aix-gap` as a valid issue kind, routed
through the same validation, redaction, and ContextBundle capture as the
existing kinds. An `aix-gap` report SHALL be convertible to a regression
Scenario via the existing `from_feedback_context` path.

#### Scenario: aix-gap kind is accepted

- **WHEN** feedback is filed with `--kind aix-gap`
- **THEN** the kind SHALL pass validation and the report SHALL flow through
  the standard feedback pipeline

#### Scenario: aix-gap near-miss gets a typo suggestion

- **WHEN** feedback is filed with kind `aix_gap` or `aixgap`
- **THEN** the suggestion engine SHALL propose `aix-gap`

#### Scenario: aix-gap capture converts to a scenario

- **WHEN** an aix-gap report captures a ContextBundle for a stale-artifact
  failure
- **THEN** `from_feedback_context` SHALL convert the bundle into a
  replayable Scenario exactly as for other kinds

