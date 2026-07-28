## 1. ConfigError type
- [x] 1.1 Define `ConfigError` enum (MissingFile, ParseError, ValidationError, IoError, TypeMismatch).
- [x] 1.2 Implement `Display` + `Error` + `From<std::io::Error>` + `From<toml::de::Error>`.
- [x] 1.3 Each variant carries a `to_suggestion() -> Suggestion` method.
- [x] 1.4 Unit tests for each variant's suggestion.

## 2. ConfigFile trait
- [x] 2.1 Define `ConfigFile` trait: `fn path()`, `fn read()`, `fn write()` (with default impl using serde).
- [x] 2.2 Provide blanket impl for `T: DeserializeOwned + Serialize + Default`.
- [x] 2.3 Add `fn validate()` with default no-op returning `Vec<ConfigValidation>`.
- [x] 2.4 Unit tests with a mock config struct.

## 3. ConfigRegistry
- [x] 3.1 Define `ConfigRegistry` using factory pattern: stores `HashMap<&'static str, ConfigEntry>`.
- [x] 3.2 `register<T: ConfigFile + DeserializeOwned + 'static>(tool_name, marker)`.
- [x] 3.3 `get(tool_name, repo_root) -> Result<Box<dyn Any>>` — calls the factory.
- [x] 3.4 `registered_tools() -> Vec<&'static str>` — list all registered tools.
- [x] 3.5 Unit tests for registration lifecycle.

## 4. ConfigStore
- [x] 4.1 `ConfigStore::new(registry)` — wraps a registry.
- [x] 4.2 `ConfigStore::discover(repo_root)` — walks markers, returns all found configs.
- [x] 4.3 `ConfigStore::validate_all()` — runs validate on each registered + found config.
- [x] 4.4 `ConfigStore::get<T>(tool_name)` — typed access.
- [x] 4.5 `ConfigStore::managed_block(repo_root) -> String` — generates the config managed block table.
- [x] 4.6 Integration tests with temp dir + real config files.

## 5. Downstream migration
- [x] 5.1 File per-repo adoption issues: per-repo `upgrade-genesis` proposals created.
- [ ] 5.2 Each tool thins its `src/config.rs` to just the struct + `ConfigFile` impl.
- [ ] 5.3 Each tool registers with `ConfigRegistry` at startup.
- [ ] 5.4 Remove dead config code from each tool.