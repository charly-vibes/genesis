# Change: Add `config` module (shared config management)

## Why

Every tool in the suite independently reads its own config — hardcoded path,
hardcoded parse, hardcoded error handling. The duplication is visible in
every `src/config.rs`:

```
wai:         Config::load(.wai/config.toml) + ~/.config/wai/config.toml
dont:        serde::from_str(read(.dont/config.toml))
pretender:   (no src/ yet)
espectacular: read(.espectacular/config.toml) → raw string replace
testaruda:   serde::from_str(read(testaruda.toml))
```

Five tools, five implementations of the same pattern. A new tool (crua,
livin, vampiro) has to write a sixth. This change extracts the shared
pattern into genesis so each tool writes 0 lines of config infra — just
a `struct Config` with derive macros.

The goal: **thinning the tools**. Each tool keeps only its domain-specific
config fields. Genesis provides the read/write/discover/validate machinery.

## What Changes

### `genesis::config` module

- **`ConfigFile` trait** — `fn path() -> PathBuf` (relative to repo root),
  `fn read() -> Result<Self, ConfigError>`, `fn write(&self) -> Result`.
  Tools derive or implement this for their config struct.

- **`ConfigRegistry`** — tools register `(tool_name, ConfigFile impl)` at
  startup. Layered (runtime), not compile-time, so the registry works across
  tools without adding a static dep on every config type known at build time.

- **`ConfigStore`** — wraps the registry. Key methods:
  - `ConfigStore::discover(repo_root)` — walks known markers (`.wai/`,
    `.dont/`, `.espectacular/`, `testaruda.toml`, `pretender.toml`) and
    returns all configs found. This is the `wai doctor --suite` primitive.
  - `ConfigStore::get<T: ConfigFile>()` — typed access to a single config.
  - `ConfigStore::validate_all()` — runs each registered config's validator.

- **`ConfigError`** — unified error type (missing file, parse error,
  validation error, IO error). Tools don't define their own.

- **Standard discovery** — `ConfigStore::discover()` knows the canonical
  paths by marker:
  ```
  .wai/             → .wai/config.toml
  .dont/            → .dont/config.toml
  .espectacular/    → .espectacular/config.toml
  pretender.toml    → pretender.toml
  testaruda.toml    → testaruda.toml
  ```

  Paths are NOT hardcoded in genesis as tool-specific logic — they're
  registered by each tool via the `ConfigRegistry`. The `discover()` method
  looks for all registered markers and calls the appropriate `ConfigFile`
  impl.

- **Validation convention** — each `ConfigFile` impl can provide a
  `validate()` method. Genesis calls it after read. Standard validations
  (unknown fields via `#[serde(deny_unknown_fields)]`, required fields,
  range checks) don't need custom code.

### What stays in each tool

Each tool keeps:
- Its own `struct Config` with domain-specific fields
- The `ConfigFile` impl (trait, not struct — just `fn path()` + derive)
- Its own `init` subcommand (calls `ConfigFile::write()`)
- Domain-specific validation (e.g. testaruda's adapter schema)

### What moves to genesis

- Config file reading (path resolution, `fs::read_to_string`, serde parse)
- Config file writing (serde serialize, `fs::write`)
- Discovery (walking repo markers → finding all configs)
- Error types (missing file, parse error, validation, IO)
- `ConfigRegistry` (runtime registration of tool configs)
- `ConfigStore` (unified access + validation)

### Boundary rule

A module is accepted only if >=2 tools need it. Config management is used
by every tool in the suite (5 shipped + 3 spec-stage). It passes the
boundary rule.

## Impact

- **New capability**: `config` — shared config management.
- **Affected code**: new `src/config/` module in genesis. Downstream tools
  delete their `src/config.rs` (or thin it to just the struct + trait impl).
- **Blocked by**: genesis v0.1.0 already tagged (config can be v0.2.0 or
  added to the existing crate).
- **Migration path**: additive — tools can adopt one at a time. Old
  `src/config.rs` can coexist during migration.