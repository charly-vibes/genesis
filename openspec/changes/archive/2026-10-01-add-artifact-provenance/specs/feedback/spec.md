# feedback Specification (delta)

## ADDED Requirements

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
