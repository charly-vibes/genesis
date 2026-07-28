# test delta

## ADDED Requirements

### Requirement: Fixture builder

genesis SHALL provide a `Fixture` builder for creating scratch test environments.

#### Scenario: with_marker creates a directory

- **WHEN** `Fixture::new().with_marker(".wai").build()` is called
- **THEN** a directory SHALL exist at `<fixture_root>/.wai/`

#### Scenario: with_file creates a file with content

- **WHEN** `Fixture::new().with_file("src/main.rs", "fn main() {}").build()` is called
- **THEN** a file SHALL exist at `<fixture_root>/src/main.rs`
- **AND** its content SHALL be `"fn main() {}"`

#### Scenario: with_toml writes a valid TOML file

- **WHEN** `Fixture::new().with_toml("config.toml", toml_value).build()` is called
- **THEN** a file SHALL exist at `<fixture_root>/config.toml`
- **AND** the file SHALL be valid TOML

#### Scenario: with_git_init creates a git repo

- **WHEN** `Fixture::new().with_git_init().build()` is called
- **THEN** `<fixture_root>/.git/` SHALL exist
- **AND** `git status` SHALL succeed inside the fixture

#### Scenario: Fixture path resolves relative paths

- **WHEN** `Fixture::new().with_marker(".wai").build()` is called
- **AND** `fixture.path(".wai/config.toml")` is called
- **THEN** the result SHALL be `<fixture_root>/.wai/config.toml`

#### Scenario: Fixture Drop cleans up the temp directory

- **WHEN** a `Fixture` goes out of scope
- **THEN** the temp directory SHALL be deleted

### Requirement: Fixture assertions

genesis SHALL provide assertion helpers for fixture content.

#### Scenario: assert_file_exists passes for existing file

- **WHEN** `fixture.assert_file_exists("src/main.rs")` is called
- **AND** the file exists
- **THEN** the assertion SHALL pass

#### Scenario: assert_file_contains passes for matching content

- **WHEN** `fixture.assert_file_contains("llm.txt", "genesis")` is called
- **AND** the file has content containing "genesis"
- **THEN** the assertion SHALL pass

### Requirement: Fixture::run() dogfooding

genesis SHALL provide a way to run a command inside the fixture.

#### Scenario: run executes a command in the fixture dir

- **WHEN** `fixture.run("echo hello")` is called
- **THEN** the command SHALL run with `<fixture_root>` as the working directory
- **AND** `output.stdout` SHALL contain "hello"

#### Scenario: run returns exit code

- **WHEN** `fixture.run("false")` is called
- **THEN** `output.exit_code` SHALL be non-zero