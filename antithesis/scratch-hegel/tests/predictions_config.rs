//! genesis-azy predictions: the config seam (P3). Each test mirrors the
//! scratchbook property config-file-round-trip-is-total; the pass/fail
//! outcome is the "predicted red/green" evidence recorded there.

mod common;

use common::temp_root;
use genesis::config::ConfigFile;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct AllDefault {
    #[serde(skip_serializing_if = "Option::is_none")]
    opt: Option<String>,
}

impl AllDefault {
    /// Consumers must join repo_root themselves: the trait doc says "relative
    /// to repo_root", but the provided read()/write() never join it. See P3a.
    fn join(repo_root: &Path) -> std::path::PathBuf {
        repo_root.join("tool.toml")
    }
}

impl ConfigFile for AllDefault {
    fn path(repo_root: &Path) -> std::path::PathBuf {
        Self::join(repo_root)
    }
}

/// The doc-faithful reading: path() returns the file relative to repo_root.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct RelativePathConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    opt: Option<String>,
}

impl ConfigFile for RelativePathConfig {
    fn path(_repo_root: &Path) -> std::path::PathBuf {
        std::path::PathBuf::from("tool.toml")
    }
}

#[test]
fn p3_config_roundtrip_all_default() {
    let root = temp_root("config");
    let cfg = AllDefault::default();
    common::require(AllDefault::write(&cfg, root.path()), "write default config");
    let bytes = common::require(std::fs::read_to_string(AllDefault::join(root.path())), "read written default config");
    eprintln!("P3 written bytes: {bytes:?}");
    match AllDefault::read(root.path()) {
        Ok(v) => {
            eprintln!("P3 re-read: {v:?}");
            assert_eq!(cfg, v, "roundtrip value mismatch");
            println!("P3 RESULT: GREEN");
        }
        Err(e) => {
            eprintln!("P3 read error: {e:?}");
            panic!("P3 RESULT: RED — write() output not readable back: {e}");
        }
    }
}

/// The documented contract: path() returns the file "relative to
/// repo_root". A consumer coding to the doc writes into the project;
/// the provided write() actually resolves against the process cwd.
#[test]
fn p3a_write_honors_repo_root() {
    let root = temp_root("config-root");
    let cfg = RelativePathConfig { opt: Some("x".into()) };
    common::require(RelativePathConfig::write(&cfg, root.path()), "write relative-path config");
    let in_project = root.path().join("tool.toml").exists();
    let in_cwd = Path::new("tool.toml").exists();
    let _ = std::fs::remove_file("tool.toml"); // cleanup the stray
    assert!(
        in_project,
        "P3a RESULT: RED — write(repo_root) ignored repo_root (documented relative-path contract)"
    );
    assert!(!in_cwd, "write leaked into the process cwd");
    println!("P3a RESULT: GREEN (repo_root honored)");
}

#[test]
fn p3_config_roundtrip_nonempty() {
    let root = temp_root("config2");
    let cfg = AllDefault {
        opt: Some("hello".into()),
    };
    common::require(AllDefault::write(&cfg, root.path()), "write default config");
    let v = common::require(AllDefault::read(root.path()), "read nonempty config back");
    assert_eq!(cfg, v);
    // write→read→write byte stability
    let file = AllDefault::join(root.path());
    let b1 = common::require(std::fs::read_to_string(&file), "read config bytes 1st pass (write→read→write byte stability)");
    common::require(AllDefault::write(&v, root.path()), "rewrite nonempty config");
    let b2 = common::require(std::fs::read_to_string(&file), "read config bytes 2nd pass");
    assert_eq!(b1, b2, "write→read→write not byte-stable");
    println!("P3b RESULT: GREEN (nonempty configs roundtrip)");
}
