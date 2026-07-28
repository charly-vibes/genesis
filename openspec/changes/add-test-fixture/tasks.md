## 1. Fixture builder
- [ ] 1.1 Define `Fixture` struct with builder pattern.
- [ ] 1.2 `with_marker(name)` — creates a directory marker.
- [ ] 1.3 `with_marker_dir(path)` — creates nested directory.
- [ ] 1.4 `with_file(path, content)` — writes a file at relative path.
- [ ] 1.5 `with_toml(path, value)` — writes a serialized TOML file.
- [ ] 1.6 `with_config<T: ConfigFile>(tool, config)` — writes config via ConfigFile trait.
- [ ] 1.7 `with_git_init()` — git init + initial commit.
- [ ] 1.8 `build()` — returns the Fixture, ready for use.
- [ ] 1.9 `Fixture` implements `Drop` to clean up the temp dir.
- [ ] 1.10 `Fixture::path(&self, relative: &str) -> PathBuf` — resolve relative path.

## 2. Fixture assertions
- [ ] 2.1 `assert_file_exists(path)` — panics if file doesn't exist.
- [ ] 2.2 `assert_file_contains(path, pattern)` — panics if content doesn't match.
- [ ] 2.3 `assert_marker(name)` — panics if marker directory doesn't exist.
- [ ] 2.4 `assert_no_file(path)` — panics if file exists.

## 3. Fixture::run() — dogfooding
- [ ] 3.1 `Fixture::run(args: &[&str])` — runs command as subprocess in fixture dir.
- [ ] 3.2 Return `Output` struct with stdout, stderr, exit_code.
- [ ] 3.3 `Output::json<T>()` — parse stdout as JSON.
- [ ] 3.4 `Output::success()` — returns true if exit_code == 0.
- [ ] 3.5 Handle PATH lookup for the tool binary.

## 4. Integration tests
- [ ] 4.1 Test with_marker creates the directory.
- [ ] 4.2 Test with_file creates and writes the file.
- [ ] 4.3 Test with_toml creates a valid TOML file.
- [ ] 4.4 Test with_git_init creates a git repo.
- [ ] 4.5 Test path resolution works correctly.
- [ ] 4.6 Test Drop cleanup removes the temp dir.
- [ ] 4.7 Test assertions pass/fail correctly.