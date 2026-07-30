//! Tool discovery via `.genesis/tools.toml` manifest.
//!
//! Provides a convention-based mechanism for genesis-based tools to declare
//! their presence in a project, so that orchestration tools like wai can
//! discover them without hardcoding a list.
//!
//! ## Convention
//!
//! Tools that use genesis write their metadata to `.genesis/tools.toml` at
//! the project root during their `init` command. The manifest is a simple
//! TOML file:
//!
//! ```toml
//! [tools.wai]
//! description = "Workflow manager for AI-driven development"
//! detector = { type = "directory", path = ".wai" }
//!
//! [tools.dont]
//! description = "Decision-logged conventions"
//! detector = { type = "directory", path = ".dont" }
//! ```
//!
//! ## Usage
//!
//! ```rust,no_run
//! use std::path::Path;
//! use genesis::discovery::{scan, register};
//!
//! let project = Path::new("/path/to/project");
//!
//! // Wai calls this instead of a hardcoded list:
//! let tools = scan(project);
//! for tool in &tools {
//!     println!("detected: {} (detected: {})", tool.name, tool.detected);
//! }
//!
//! // A tool registers itself during init:
//! register(project, "my-tool", "My tool description",
//!     "directory", ".my-tool").unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// The filename for the genesis tool manifest, relative to the project root.
pub const MANIFEST_FILENAME: &str = ".genesis/tools.toml";

/// Full manifest file contents.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    /// Map of tool name to tool definition.
    #[serde(default)]
    pub tools: HashMap<String, ToolEntry>,
}

/// A single tool entry in the manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    /// Human-readable description of the tool.
    #[serde(default)]
    pub description: String,
    /// How to detect whether this tool is active in a project.
    #[serde(default)]
    pub detector: DetectorDef,
}

/// How to detect a tool's presence in a project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorDef {
    /// Detection strategy: `"directory"` or `"file"`.
    #[serde(rename = "type")]
    pub detector_type: String,
    /// Path relative to the project root (e.g. `".wai"`, `"testaruda.toml"`).
    pub path: String,
}

impl Default for DetectorDef {
    fn default() -> Self {
        Self {
            detector_type: "directory".to_string(),
            path: String::new(),
        }
    }
}

/// A tool that was detected (or not) in a project.
#[derive(Debug, Clone)]
pub struct DetectedTool {
    /// The tool name (e.g. `"wai"`, `"dont"`).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the tool's detector marker was found in the project.
    pub detected: bool,
}

/// Read the manifest file from a project root.
///
/// Returns `None` if the file doesn't exist or can't be parsed.
pub fn read_manifest(project_root: &Path) -> Option<Manifest> {
    let path = project_root.join(MANIFEST_FILENAME);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    toml::from_str(&content).ok()
}

/// Scan a project root for all genesis-based tools defined in the manifest.
///
/// Returns an empty vec if the manifest doesn't exist (no error — it's
/// normal for projects that don't use genesis tools yet).
pub fn scan(project_root: &Path) -> Vec<DetectedTool> {
    let manifest = match read_manifest(project_root) {
        Some(m) => m,
        None => return Vec::new(),
    };

    let mut tools = Vec::new();
    for (name, entry) in &manifest.tools {
        let detected = if entry.detector.path.is_empty() {
            false
        } else {
            let marker_path = project_root.join(&entry.detector.path);
            match entry.detector.detector_type.as_str() {
                "directory" => marker_path.is_dir(),
                "file" => marker_path.is_file(),
                other => {
                    // Unknown detector type — treat as not detected
                    log_unknown_detector(other, name);
                    false
                }
            }
        };
        tools.push(DetectedTool {
            name: name.clone(),
            description: entry.description.clone(),
            detected,
        });
    }

    tools
}

/// Register a tool in the genesis manifest.
///
/// Creates the `.genesis/` directory and `tools.toml` file if they don't
/// exist, and adds/updates the tool entry.
///
/// Tools should call this during their `init` command.
pub fn register(
    project_root: &Path,
    name: &str,
    description: &str,
    detector_type: &str,
    detector_path: &str,
) -> Result<(), String> {
    let manifest_path = project_root.join(MANIFEST_FILENAME);

    // Ensure parent directory exists
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create .genesis/ directory: {e}"))?;
    }

    // Read existing manifest or start fresh
    let mut manifest = read_manifest(project_root).unwrap_or_default();

    // Add/update the tool entry
    manifest.tools.insert(
        name.to_string(),
        ToolEntry {
            description: description.to_string(),
            detector: DetectorDef {
                detector_type: detector_type.to_string(),
                path: detector_path.to_string(),
            },
        },
    );

    // Write back
    let content = toml::to_string_pretty(&manifest)
        .map_err(|e| format!("failed to serialize manifest: {e}"))?;
    std::fs::write(&manifest_path, &content)
        .map_err(|e| format!("failed to write manifest: {e}"))?;

    Ok(())
}

/// Remove a tool from the genesis manifest.
///
/// Useful for `deinit` or uninstall workflows.
pub fn unregister(project_root: &Path, name: &str) -> Result<(), String> {
    let manifest_path = project_root.join(MANIFEST_FILENAME);
    let mut manifest = match read_manifest(project_root) {
        Some(m) => m,
        None => return Ok(()), // Nothing to remove
    };

    manifest.tools.remove(name);

    if manifest.tools.is_empty() {
        // Last tool removed — delete the file
        std::fs::remove_file(&manifest_path)
            .map_err(|e| format!("failed to remove manifest: {e}"))?;
        // Try to clean up empty .genesis/ directory (best-effort)
        if let Some(parent) = manifest_path.parent() {
            let _ = std::fs::remove_dir(parent);
        }
    } else {
        let content = toml::to_string_pretty(&manifest)
            .map_err(|e| format!("failed to serialize manifest: {e}"))?;
        std::fs::write(&manifest_path, &content)
            .map_err(|e| format!("failed to write manifest: {e}"))?;
    }

    Ok(())
}

/// List all tool names registered in the manifest.
pub fn list_tools(project_root: &Path) -> Vec<String> {
    match read_manifest(project_root) {
        Some(m) => m.tools.into_keys().collect(),
        None => Vec::new(),
    }
}

/// Check if the genesis manifest exists at the project root.
pub fn has_manifest(project_root: &Path) -> bool {
    project_root.join(MANIFEST_FILENAME).exists()
}

fn log_unknown_detector(detector_type: &str, tool_name: &str) {
    // Best-effort stderr note — not a hard error so tools degrade gracefully.
    let _ = writeln!(
        std::io::stderr(),
        "genesis: unknown detector type '{}' for tool '{}' (expected 'directory' or 'file')",
        detector_type,
        tool_name,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    // ── read_manifest ──────────────────────────────────────────────────

    #[test]
    fn test_read_manifest_nonexistent() {
        let dir = tmp();
        assert!(read_manifest(dir.path()).is_none());
    }

    #[test]
    fn test_read_manifest_valid() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"[tools.wai]
description = "Workflow manager"
detector = { type = "directory", path = ".wai" }

[tools.dont]
description = "Conventions"
detector = { type = "directory", path = ".dont" }
"#,
        )
        .unwrap();

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.tools.len(), 2);
        assert!(manifest.tools.contains_key("wai"));
        assert!(manifest.tools.contains_key("dont"));
        assert_eq!(manifest.tools["wai"].detector.path, ".wai");
    }

    // ── scan ───────────────────────────────────────────────────────────

    #[test]
    fn test_scan_no_manifest() {
        let dir = tmp();
        let tools = scan(dir.path());
        assert!(tools.is_empty());
    }

    #[test]
    fn test_scan_detects_directory() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Create the marker directory
        std::fs::create_dir(dir.path().join(".wai")).unwrap();

        std::fs::write(
            &path,
            r#"[tools.wai]
description = "Workflow manager"
detector = { type = "directory", path = ".wai" }
"#,
        )
        .unwrap();

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "wai");
        assert!(tools[0].detected);
    }

    #[test]
    fn test_scan_not_detected_when_missing() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"[tools.wai]
description = "Workflow manager"
detector = { type = "directory", path = ".wai" }
"#,
        )
        .unwrap();

        // No .wai directory — should show as not detected
        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "wai");
        assert!(!tools[0].detected);
    }

    #[test]
    fn test_scan_detects_file() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Create the marker file
        std::fs::write(dir.path().join("testaruda.toml"), "").unwrap();

        std::fs::write(
            &path,
            r#"[tools.testaruda]
description = "Test harness"
detector = { type = "file", path = "testaruda.toml" }
"#,
        )
        .unwrap();

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1);
        assert!(tools[0].detected);
    }

    #[test]
    fn test_scan_unknown_detector_type() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r#"[tools.foo]
description = "Unknown detector"
detector = { type = "magic", path = ".foo" }
"#,
        )
        .unwrap();

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 1);
        assert!(!tools[0].detected, "unknown detector type = not detected");
    }

    // ── register ──────────────────────────────────────────────────────

    #[test]
    fn test_register_creates_manifest() {
        let dir = tmp();
        register(dir.path(), "my-tool", "My tool", "directory", ".my-tool").unwrap();

        let path = dir.path().join(MANIFEST_FILENAME);
        assert!(path.is_file(), "manifest should be created");

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.tools["my-tool"].description, "My tool");
        assert_eq!(manifest.tools["my-tool"].detector.path, ".my-tool");
    }

    #[test]
    fn test_register_appends_to_existing() {
        let dir = tmp();
        register(dir.path(), "tool-a", "A", "directory", ".a").unwrap();
        register(dir.path(), "tool-b", "B", "file", "b.toml").unwrap();

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.tools.len(), 2);
    }

    #[test]
    fn test_register_updates_existing() {
        let dir = tmp();
        register(dir.path(), "tool", "Old description", "directory", ".old").unwrap();
        register(dir.path(), "tool", "New description", "file", "new.toml").unwrap();

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.tools.len(), 1);
        assert_eq!(manifest.tools["tool"].description, "New description");
        assert_eq!(manifest.tools["tool"].detector.path, "new.toml");
        assert_eq!(manifest.tools["tool"].detector.detector_type, "file");
    }

    // ── unregister ────────────────────────────────────────────────────

    #[test]
    fn test_unregister_removes_entry() {
        let dir = tmp();
        register(dir.path(), "tool-a", "A", "directory", ".a").unwrap();
        register(dir.path(), "tool-b", "B", "directory", ".b").unwrap();

        unregister(dir.path(), "tool-a").unwrap();

        let manifest = read_manifest(dir.path()).unwrap();
        assert_eq!(manifest.tools.len(), 1);
        assert!(manifest.tools.contains_key("tool-b"));
    }

    #[test]
    fn test_unregister_removes_file_when_empty() {
        let dir = tmp();
        register(dir.path(), "tool", "Only tool", "directory", ".t").unwrap();
        unregister(dir.path(), "tool").unwrap();

        let path = dir.path().join(MANIFEST_FILENAME);
        assert!(!path.exists(), "manifest should be deleted when empty");
    }

    #[test]
    fn test_unregister_nonexistent_is_noop() {
        let dir = tmp();
        // No manifest at all — should not error
        unregister(dir.path(), "ghost").unwrap();
    }

    // ── list_tools ────────────────────────────────────────────────────

    #[test]
    fn test_list_tools_empty() {
        let dir = tmp();
        assert!(list_tools(dir.path()).is_empty());
    }

    #[test]
    fn test_list_tools_returns_names() {
        let dir = tmp();
        register(dir.path(), "alpha", "A", "directory", ".a").unwrap();
        register(dir.path(), "beta", "B", "directory", ".b").unwrap();

        let mut names = list_tools(dir.path());
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    // ── has_manifest ──────────────────────────────────────────────────

    #[test]
    fn test_has_manifest_false() {
        let dir = tmp();
        assert!(!has_manifest(dir.path()));
    }

    #[test]
    fn test_has_manifest_true() {
        let dir = tmp();
        register(dir.path(), "t", "t", "directory", ".t").unwrap();
        assert!(has_manifest(dir.path()));
    }

    // ── DetectedTool ──────────────────────────────────────────────────

    #[test]
    fn test_scan_multiple_tools() {
        let dir = tmp();
        let path = dir.path().join(MANIFEST_FILENAME);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        // Create marker dirs
        std::fs::create_dir(dir.path().join(".present")).unwrap();

        std::fs::write(
            &path,
            r#"[tools.present]
detector = { type = "directory", path = ".present" }

[tools.absent]
detector = { type = "directory", path = ".absent" }
"#,
        )
        .unwrap();

        let tools = scan(dir.path());
        assert_eq!(tools.len(), 2);

        let present = tools.iter().find(|t| t.name == "present").unwrap();
        assert!(present.detected);

        let absent = tools.iter().find(|t| t.name == "absent").unwrap();
        assert!(!absent.detected);
    }
}
