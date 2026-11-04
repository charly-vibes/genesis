//! Unit tests for `scaffold` (extracted from the inline `mod tests` to
//! keep the app file under the pretender file-lines budget).

// pretender: role=test
#![allow(unused_imports)]

use super::*;

use crate::testutil::TestUnwrap;
use tempfile::TempDir;

fn tmp() -> TempDir {
    tempfile::tempdir().require("tempdir")
}

fn project_root(dir: &TempDir) -> PathBuf {
    dir.path().join("my-project")
}

// ── Basic scaffolding ─────────────────────────────────────────────

#[test]
fn test_create_directories() {
    let dir = tmp();
    let root = project_root(&dir);

    let result = Scaffold::new(&root)
        .dir(".tool/state")
        .dir(".tool/cache/sub")
        .build()
        .require("value");

    assert!(root.join(".tool/state").is_dir(), "state dir should exist");
    assert!(
        root.join(".tool/cache/sub").is_dir(),
        "nested cache dir should exist"
    );
    assert_eq!(result.created.len(), 2, "both dirs should be created");
}

#[test]
fn test_skip_existing_directory() {
    let dir = tmp();
    let root = project_root(&dir);
    std::fs::create_dir_all(root.join(".tool")).require("value");

    let result = Scaffold::new(&root).dir(".tool").build().require("value");

    assert!(
        result.created.is_empty(),
        "existing dir should not be recreated"
    );
    assert_eq!(result.existed.len(), 1, "existing dir should be in existed");
}

#[test]
fn test_write_default_config() {
    let dir = tmp();
    let root = project_root(&dir);

    let result = Scaffold::new(&root)
        .default_config("config.toml", "key = \"value\"\n")
        .build()
        .require("value");

    let config_path = root.join("config.toml");
    assert!(config_path.is_file(), "config file should exist");
    let content = std::fs::read_to_string(&config_path).require("value");
    assert_eq!(content, "key = \"value\"\n");
    assert_eq!(result.created.len(), 1);
}

#[test]
fn test_skip_existing_config() {
    let dir = tmp();
    let root = project_root(&dir);
    std::fs::create_dir_all(&root).require("value");
    std::fs::write(root.join("config.toml"), "original").require("value");

    let result = Scaffold::new(&root)
        .default_config("config.toml", "new content")
        .build()
        .require("value");

    // Content should not be overwritten
    let content = std::fs::read_to_string(root.join("config.toml")).require("value");
    assert_eq!(
        content, "original",
        "existing config should not be overwritten"
    );
    assert!(result.created.is_empty(), "nothing should be created");
    assert_eq!(
        result.existed.len(),
        1,
        "existing file should be in existed"
    );
}

#[test]
fn test_create_nested_config_creates_parent_dir() {
    let dir = tmp();
    let root = project_root(&dir);

    let result = Scaffold::new(&root)
        .default_config("deep/nested/config.toml", "data")
        .build()
        .require("value");

    assert!(root.join("deep/nested/config.toml").is_file());
    assert_eq!(result.created.len(), 1);
}

// ── .gitignore ────────────────────────────────────────────────────

#[test]
fn test_gitignore_entry_new_file() {
    let dir = tmp();
    let root = project_root(&dir);

    Scaffold::new(&root)
        .gitignore_entry(".tool/cache/")
        .build()
        .require("value");

    let gitignore = root.join(".gitignore");
    assert!(gitignore.is_file(), ".gitignore should be created");
    let content = std::fs::read_to_string(&gitignore).require("value");
    assert!(
        content.contains(".tool/cache/"),
        "gitignore should contain entry"
    );
}

#[test]
fn test_gitignore_append_to_existing() {
    let dir = tmp();
    let root = project_root(&dir);
    std::fs::create_dir_all(&root).require("value");
    std::fs::write(root.join(".gitignore"), "node_modules/\n").require("value");

    Scaffold::new(&root)
        .gitignore_entry(".tool/cache/")
        .gitignore_entry("target/")
        .build()
        .require("value");

    let content = std::fs::read_to_string(root.join(".gitignore")).require("value");
    assert!(
        content.contains("node_modules/"),
        "original content preserved"
    );
    assert!(content.contains(".tool/cache/"), "new entry added");
    assert!(content.contains("target/"), "second new entry added");
}

#[test]
fn test_gitignore_skip_duplicate() {
    let dir = tmp();
    let root = project_root(&dir);
    std::fs::create_dir_all(&root).require("value");
    std::fs::write(root.join(".gitignore"), ".tool/cache/\n").require("value");

    Scaffold::new(&root)
        .gitignore_entry(".tool/cache/")
        .build()
        .require("value");

    let content = std::fs::read_to_string(root.join(".gitignore")).require("value");
    // Should only appear once
    assert_eq!(content.matches(".tool/cache/").count(), 1);
}

// ── Managed block ─────────────────────────────────────────────────

#[test]
fn test_managed_block() {
    let dir = tmp();
    let root = project_root(&dir);

    Scaffold::new(&root)
        .managed_block("MYTOOL", "config.toml", "# My Tool Config\nkey = \"value\"")
        .build()
        .require("value");

    let content = std::fs::read_to_string(root.join("config.toml")).require("value");
    assert!(content.contains("<!-- MYTOOL:START -->"));
    assert!(content.contains("<!-- MYTOOL:END -->"));
    assert!(content.contains("My Tool Config"));
}

// ── Agent command file ────────────────────────────────────────────

#[test]
fn test_agent_command_file_creates_parent_dir() {
    let dir = tmp();
    let root = project_root(&dir);

    Scaffold::new(&root)
        .agent_command_file(".claude/commands/my-tool.md", "# My Tool\n\nUsage")
        .build()
        .require("value");

    let path = root.join(".claude/commands/my-tool.md");
    assert!(path.is_file(), "agent command file should exist");
    let content = std::fs::read_to_string(&path).require("value");
    assert_eq!(content, "# My Tool\n\nUsage");
}

#[test]
fn test_multiple_operations() {
    let dir = tmp();
    let root = project_root(&dir);

    let result = Scaffold::new(&root)
        .dir(".tool/state")
        .dir(".tool/cache")
        .default_config("config.toml", "enabled = true\n")
        .gitignore_entry(".tool/cache/")
        .agent_command_file(".claude/commands/my-tool.md", "# command")
        .build()
        .require("value");

    assert!(result.created.len() >= 4, "should create multiple items");
    assert!(root.join(".tool/state").is_dir());
    assert!(root.join(".tool/cache").is_dir());
    assert!(root.join("config.toml").is_file());
    assert!(root.join(".claude/commands/my-tool.md").is_file());
    assert!(root.join(".gitignore").is_file());
}
