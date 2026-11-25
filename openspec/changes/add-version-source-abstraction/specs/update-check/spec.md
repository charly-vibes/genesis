# update-check delta

## ADDED Requirements

### Requirement: Publisher abstraction

The module SHALL resolve a published version through a named publisher source.
The supported sources SHALL be the crates.io registry, a GitHub repository's
latest release, and the npm registry. Each source SHALL be identified by the
coordinate it needs.

#### Scenario: a crates.io coordinate resolves to a version

- **WHEN** a crates.io source is asked for the latest version of a crate
- **THEN** it SHALL return the published version string
- **AND** SHALL NOT return a value for a crate that does not exist

#### Scenario: a GitHub release coordinate resolves to a version

- **WHEN** a GitHub release source is asked for the latest version of a
      repository
- **THEN** it SHALL return the release version without a leading `v` prefix

#### Scenario: an npm coordinate resolves to a version

- **WHEN** an npm source is asked for the latest version of a package
- **THEN** it SHALL return the published version string

#### Scenario: an unreachable publisher yields no version

- **WHEN** the publisher cannot be reached or answers with an error
- **THEN** the source SHALL return no version
- **AND** SHALL NOT panic

### Requirement: One comparison path

The module SHALL compare an installed version against a published version
through one comparison path, so that every publisher's answer is judged the
same way.

#### Scenario: an installed version behind the published version is outdated

- **WHEN** the installed version is lower than the published version
- **THEN** the comparison SHALL report the installed version as outdated

#### Scenario: an installed version equal to the published version is current

- **WHEN** the installed version equals the published version
- **THEN** the comparison SHALL report the installed version as current

#### Scenario: a pre-release installed version is not outdated

- **WHEN** the installed version is a pre-release of the published version
- **THEN** the comparison SHALL report the installed version as current

### Requirement: Sources stay opt-in

Resolving a published version SHALL remain behind the opt-in feature. A
dependable that does not enable the feature SHALL perform no network access
through this module, and the HTTP stack SHALL NOT be compiled into it.

#### Scenario: a dependent without the feature performs no lookup

- **WHEN** a dependent builds without the `update-check` feature
- **THEN** the HTTP stack SHALL NOT be part of the dependency graph
- **AND** no version lookup SHALL be possible
