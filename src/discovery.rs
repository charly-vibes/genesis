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
    let Some(manifest) = read_manifest(project_root) else {
        return Vec::new();
    };
    manifest
        .tools
        .iter()
        .map(|(name, entry)| detect_tool(project_root, name, entry))
        .collect()
}

/// Probe one manifest entry: does its detector's marker exist on disk?
fn detect_tool(project_root: &Path, name: &str, entry: &ToolEntry) -> DetectedTool {
    DetectedTool {
        name: name.to_string(),
        description: entry.description.clone(),
        detected: marker_exists(project_root, name, &entry.detector),
    }
}

/// Does the detector's marker exist, given a non-empty relative path?
///
/// An empty path never matches; an unknown detector type logs a note and
/// reports `false` so tools degrade gracefully instead of erroring.
fn marker_exists(project_root: &Path, name: &str, detector: &DetectorDef) -> bool {
    if detector.path.is_empty() {
        return false;
    }
    let marker_path = project_root.join(&detector.path);
    match detector.detector_type.as_str() {
        "directory" => marker_path.is_dir(),
        "file" => marker_path.is_file(),
        other => {
            log_unknown_detector(other, name);
            false
        }
    }
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
    ensure_manifest_dir(&manifest_path)?;
    let manifest = upsert_tool(
        read_manifest(project_root).unwrap_or_default(),
        name,
        description,
        detector_type,
        detector_path,
    );
    write_manifest(&manifest_path, &manifest)
}

/// Create the manifest's parent directory (`.genesis/`) on demand.
fn ensure_manifest_dir(manifest_path: &Path) -> Result<(), String> {
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create .genesis/ directory: {e}"))?;
    }
    Ok(())
}

/// Insert or replace `name` in the manifest's tool table.
fn upsert_tool(
    mut manifest: Manifest,
    name: &str,
    description: &str,
    detector_type: &str,
    detector_path: &str,
) -> Manifest {
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
    manifest
}

/// Serialize the manifest to TOML and write it to `manifest_path`.
fn write_manifest(manifest_path: &Path, manifest: &Manifest) -> Result<(), String> {
    let content = toml::to_string_pretty(manifest)
        .map_err(|e| format!("failed to serialize manifest: {e}"))?;
    std::fs::write(manifest_path, &content)
        .map_err(|e| format!("failed to write manifest: {e}"))?;
    Ok(())
}

/// Remove a tool from the genesis manifest.
///
/// Useful for `deinit` or uninstall workflows. A missing manifest is a
/// no-op success: there is nothing to remove and none must be created.
pub fn unregister(project_root: &Path, name: &str) -> Result<(), String> {
    let Some(mut manifest) = read_manifest(project_root) else {
        return Ok(());
    };
    manifest.tools.remove(name);
    let manifest_path = project_root.join(MANIFEST_FILENAME);
    if manifest.tools.is_empty() {
        remove_manifest_and_dir(&manifest_path)
    } else {
        write_manifest(&manifest_path, &manifest)
    }
}

/// Delete the manifest file, then the `.genesis/` parent if it is now empty.
fn remove_manifest_and_dir(manifest_path: &Path) -> Result<(), String> {
    std::fs::remove_file(manifest_path).map_err(|e| format!("failed to remove manifest: {e}"))?;
    if let Some(parent) = manifest_path.parent() {
        let _ = std::fs::remove_dir(parent);
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

/// Note an unknown detector type on stderr; best-effort, never fatal.
fn log_unknown_detector(detector_type: &str, tool_name: &str) {
    let _ = writeln!(
        std::io::stderr(),
        "genesis: unknown detector type '{}' for tool '{}' (expected 'directory' or 'file')",
        detector_type,
        tool_name,
    );
}

#[cfg(test)]
#[path = "discovery/tests.rs"]
mod tests;
