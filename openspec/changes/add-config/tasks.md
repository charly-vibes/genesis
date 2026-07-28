## 1. ConfigError type
- [ ] 1.1 Define `ConfigError` enum (MissingFile, ParseError, ValidationError, IoError).
- [ ] 1.2 Implement `Display` + `Error` + `From<std::io::Error>` + `From<toml::de::Error>`.
- [ ] 1.3 Each variant carries a `to_suggestion() -> Suggestion` method:
      - MissingFile → `Suggestion::Fix { "Run <tool> init to create config", Some("<tool> init") }`
      - ValidationError → `Suggestion::Fix { "Run <tool> doctor --fix", Some("<tool> doctor --fix") }`
      - ParseError → `Suggestion::Fix { "Run <tool> doctor", Some("<tool> doctor") }`
- [ ] 1.4 Unit tests for each variant's suggestion.

## 2. ConfigFile trait
- [ ] 2.1 Define `ConfigFile` trait: `fn path(repo_root: &Path) -> PathBuf`, `fn read(repo_root: &Path) -> Result<Self>`, `fn write(&self, repo_root: &Path) -> Result` (with default impl using serde).
- [ ] 2.2 Provide blanket impl for `T: Deserialize + Serialize + Default` (no derive macro — serde already provides the derive).
- [ ] 2.3 Add `fn validate(&self) -> Result<Vec<ConfigValidation>, ConfigError>` with default no-op.
- [ ] 2.4 Unit tests with a mock config struct.

## 3. ConfigRegistry
- [ ] 3.1 Define `ConfigRegistry` using factory pattern: stores `HashMap<&'static str, ConfigEntry>` where `ConfigEntry` holds a factory fn and marker type.
- [ ] 3.2 `register<T: ConfigFile>(tool_name, marker)` — stores the factory + marker type.
- [ ] 3.3 `get(tool_name) -> Result<Box<dyn Any>>` — calls the factory, reads + parses config.
- [ ] 3.4 `registered_tools() -> Vec<&str>` — list all registered tools.
- [ ] 3.5 Unit tests for registration lifecycle.

## 4. ConfigStore
- [ ] 4.1 `ConfigStore::new(registry)` — wraps a registry.
- [ ] 4.2 `ConfigStore::discover(repo_root)` — walks markers, returns all found configs.
- [ ] 4.3 `ConfigStore::validate_all()` — runs validate on each registered + found config.
- [ ] 4.4 `ConfigStore::get<T>(tool_name)` — typed access.
- [ ] 4.5 `ConfigStore::managed_block(repo_root) -> String` — generates the config
      managed block table (tool, path, status, next-step).
- [ ] 4.6 Integration tests with temp dir + real config files.

## 5. Downstream migration
- [ ] 5.1 File per-repo adoption issues for each tool to adopt `genesis::config`.
- [ ] 5.2 Each tool thins its `src/config.rs` to just the struct + `ConfigFile` impl.
- [ ] 5.3 Each tool registers with `ConfigRegistry` at startup.
- [ ] 5.4 Remove dead config code from each tool.