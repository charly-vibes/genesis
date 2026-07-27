# genesis capability spec

## ADDED Requirements

### Requirement: Shared crate for cross-cutting CLI infrastructure

The suite SHALL provide a single shared crate, `genesis`, that owns
cross-cutting CLI, AIX, and self-healing infrastructure used by two or more
charly-vibes tools.

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

### Requirement: Boundary rule

A capability SHALL be accepted into genesis only if two or more suite
tools need it. Domain logic SHALL NOT be extracted into genesis.

#### Scenario: a proposed module serves one tool

- **WHEN** a contributor proposes a new module and only one tool consumes it
- **THEN** the proposal SHALL be rejected with a request to keep the module
  in the consuming tool.

### Requirement: Distribution

genesis SHALL be distributed as a git dependency pinned by tag from
`github.com/charly-vibes/genesis`. It SHALL NOT be published to crates.io
until its interface is tagged stable.

#### Scenario: a tool adopts genesis

- **WHEN** a tool adds genesis to its `Cargo.toml`
- **THEN** the dependency line SHALL pin a tag (e.g. `tag = "v0.1.0"`)
- **AND** SHALL NOT track a branch.

### Requirement: Versioning

genesis SHALL follow SemVer. A breaking change SHALL bump the minor
version, tag a new release, and open a coordinated change updating every
dependent's tag pin. Removed surfaces SHALL be deprecated for one release
cycle before removal.

#### Scenario: a breaking change is made to a genesis module

- **WHEN** a contributor makes a breaking change to a genesis module
- **THEN** the minor version SHALL be bumped and a new tag SHALL be cut
- **AND** a coordinated change SHALL be opened updating every dependent tool's tag pin
- **AND** the removed surface SHALL be deprecated for one release cycle before removal.
