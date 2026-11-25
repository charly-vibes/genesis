//! Full-lifecycle integration test (split from tests_store.rs for the
//! pretender file-lines budget): registry → write → discover → typed
//! read → managed block.

use super::*;

/// The second lifecycle tool: an `other.toml` with host/port.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
struct OtherConfig {
    host: String,
    port: u16,
}

impl Default for OtherConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 8080,
        }
    }
}

impl ConfigFile for OtherConfig {
    fn path(repo_root: &Path) -> PathBuf {
        repo_root.join("other.toml")
    }
}

#[test]
fn test_integration_full_lifecycle() {
    let dir = tmp();
    let mut reg = ConfigRegistry::new();
    reg.register::<MockConfig>("mock", ".mock/config.toml");
    reg.register::<OtherConfig>("other", "other.toml");
    write_lifecycle_configs(dir.path());

    let store = ConfigStore::new(reg);
    assert_all_found(&store, dir.path());
    let read_mock: MockConfig = store.get("mock", dir.path()).require("value");
    let read_other: OtherConfig = store.get("other", dir.path()).require("value");
    assert_round_trip(&read_mock, "lifecycle", &read_other, "example.com");
    assert_block_found(&store, dir.path());
}

/// Discovery must report both lifecycle tools, all found.
fn assert_all_found(store: &ConfigStore, root: &Path) {
    let discovered = ConfigStore::discover(root, store.registry());
    assert_eq!(discovered.len(), 2);
    assert!(discovered.iter().all(|d| d.found));
}

/// Both lifecycle round-trips: names must survive the write/read cycle.
fn assert_round_trip(
    mock: &MockConfig,
    expected_mock_name: &str,
    other: &OtherConfig,
    expected_host: &str,
) {
    assert_eq!(mock.name, expected_mock_name);
    assert_eq!(other.host, expected_host);
    assert_eq!(other.port, 443);
}

/// Managed block must mark both lifecycle tools found.
fn assert_block_found(store: &ConfigStore, root: &Path) {
    let block = store.managed_block(root);
    assert!(block.contains("✅ found"));
    assert!(block.contains("mock"));
    assert!(block.contains("other"));
}

/// Write both lifecycle configs under `root`.
fn write_lifecycle_configs(root: &Path) {
    MockConfig {
        name: "lifecycle".to_string(),
        version: 3,
        enabled: true,
    }
    .write(root)
    .require("write mock config");

    OtherConfig {
        host: "example.com".to_string(),
        port: 443,
    }
    .write(root)
    .require("write other config");
}
