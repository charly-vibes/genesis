# genesis delta

## MODIFIED Requirements

### Requirement: Shared crate for cross-cutting CLI infrastructure

The suite SHALL provide a single shared crate, `genesis`, that owns
cross-cutting CLI, AIX, and self-healing infrastructure used by two or
more charly-vibes tools.

#### Scenario: a tool needs the JSON envelope

- **WHEN** a tool's command emits `--json` output
- **THEN** it SHALL wrap its payload in `genesis::envelope::Envelope`
- **AND** the emitted JSON SHALL share the top-level keys
  (`ok`, `envelope_version`, `cli_version`, `envelope_kind`, `data`,
  `warnings`, `hints`, `meta`) across every tool.

#### Scenario: a tool needs self-healing errors

- **WHEN** a tool encounters an unknown subcommand or a fixable error
- **THEN** it SHALL use `genesis::suggestions::Suggestion` to emit a
  "→ Run: …" footer
- **AND** SHALL NOT emit a bare error without a fix or context hint.

#### Scenario: tools need git hook primitives

- **WHEN** a tool needs to install, detect, or wire a git hook
  (ownership-marked install/uninstall, hook owner or framework
  detection, lefthook managed-block wiring)
- **THEN** it SHALL use `genesis::git_hooks`
- **AND** SHALL NOT maintain a private copy of these primitives
