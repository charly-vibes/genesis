# managed-block (delta)

## ADDED Requirements

### Requirement: Block ownership and retirement

The managed-block contract SHALL be documented such that a block is owned by
exactly one tool, a tool that stops publishing a block removes it, and no other
tool re-creates it. A checker SHALL verify what a tool publishes — a managed
block or a published artifact path — scoped to repositories that use that tool.

#### Scenario: Injection touches only the named block

- **WHEN** an injector updates block `X` in a file that also contains blocks
  `Y` and `Z`
- **THEN** the bytes of `Y` and `Z`, their markers included, are unchanged

#### Scenario: A retired block is not re-created

- **WHEN** a tool has retired its managed block and publishes an artifact
  instead
- **THEN** no orchestrator writes the retired markers back into `AGENTS.md`
- **AND** the checker's signal for that tool is the published artifact

#### Scenario: Non-standard marker dialects are supported

- **WHEN** a tool's markers do not follow `<!-- NAME:START -->` /
  `<!-- NAME:END -->`
- **THEN** the tool can declare its markers explicitly and the injector
  recognizes them
- **AND** the documented dialect table names every form in use

#### Scenario: Drift is detectable without regenerating content

- **WHEN** an injected block gains its provenance footer
- **THEN** recomputing the content hash against the block body detects
  whether the block still matches its generator's output

#### Scenario: Facts and procedures are separated

- **WHEN** a tool moves procedural instructions out of `AGENTS.md`
- **THEN** the documented contract permits it, records where the procedures
  now live, and requires the pointer to be updated rather than the region
  re-created