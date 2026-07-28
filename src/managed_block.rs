//! Managed block injector (`<!-- …:START -->`/`<!-- …:END -->`).
//!
//! Port from `wai/src/managed_block.rs` (also used in dont/espectacular).
//!
//! ## Changes from wai
//!
//! - Generalized from wai-specific to any named block via `BlockRegistry`.
//! - `BlockDef` replaces wai's hardcoded WAI_START/WAI_END constants.
//! - `BlockInjector` replaces wai's `inject_managed_block` function.
//! - Slim-block Layer-1 behavior preserved (progressive disclosure).
//! - Removed pipeline-specific content generation (not general enough).
//! - Removed `wai_detailed_content` (tool-specific; tools generate their own).

use std::path::Path;

/// A named managed block with start/end markers.
#[derive(Debug, Clone)]
pub struct BlockDef {
    /// Display name for the block (e.g., "WAI", "DONT", "ah:managed").
    pub name: String,
    /// Start marker (e.g., `<!-- WAI:START -->`).
    pub start_marker: String,
    /// End marker (e.g., `<!-- WAI:END -->`).
    pub end_marker: String,
}

impl BlockDef {
    /// Create a new block definition from a name.
    ///
    /// Markers are automatically generated as `<!-- {name}:START -->`
    /// and `<!-- {name}:END -->`.
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start_marker: format!("<!-- {}:START -->", name),
            end_marker: format!("<!-- {}:END -->", name),
        }
    }

    /// Create a block definition with custom markers.
    ///
    /// Use this for blocks that don't follow the `<!-- NAME:START -->` convention.
    pub fn with_markers(name: &str, start_marker: &str, end_marker: &str) -> Self {
        Self {
            name: name.to_string(),
            start_marker: start_marker.to_string(),
            end_marker: end_marker.to_string(),
        }
    }
}

/// A registry of named managed blocks.
///
/// Tools register their blocks once at startup, then use `BlockInjector`
/// to inject/update/read them in files.
#[derive(Debug, Clone, Default)]
pub struct BlockRegistry {
    blocks: Vec<BlockDef>,
}

impl BlockRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a block definition.
    ///
    /// If a block with the same name already exists, it is replaced.
    pub fn register(&mut self, block: BlockDef) {
        self.blocks.retain(|b| b.name != block.name);
        self.blocks.push(block);
    }

    /// Get a block definition by name.
    pub fn get(&self, name: &str) -> Option<&BlockDef> {
        self.blocks.iter().find(|b| b.name == name)
    }

    /// Get all registered block names.
    pub fn names(&self) -> Vec<&str> {
        self.blocks.iter().map(|b| b.name.as_str()).collect()
    }

    /// Check if a block with the given name is registered.
    pub fn has(&self, name: &str) -> bool {
        self.blocks.iter().any(|b| b.name == name)
    }
}

/// Result of a block injection operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InjectResult {
    /// File was created with the block.
    Created,
    /// Block was prepended to an existing file.
    Prepended,
    /// Block was updated in-place in an existing file.
    Updated,
}

/// Injector for managed blocks.
///
/// Reads, writes, and updates managed blocks in files.
pub struct BlockInjector {
    registry: BlockRegistry,
}

impl BlockInjector {
    /// Create a new injector with the given block registry.
    pub fn new(registry: BlockRegistry) -> Self {
        Self { registry }
    }

    /// Get a reference to the block registry.
    pub fn registry(&self) -> &BlockRegistry {
        &self.registry
    }

    /// Inject a managed block into a file.
    ///
    /// If the file exists and already has the block, it is updated in-place.
    /// If the file exists but has no block, the block is prepended.
    /// If the file doesn't exist, it is created with the block.
    pub fn inject(
        &self,
        path: &Path,
        block_name: &str,
        content: &str,
    ) -> Result<InjectResult, std::io::Error> {
        let block = self.registry.get(block_name).ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("block '{}' not registered", block_name),
            )
        })?;

        let block_text = format!("{}{}{}", block.start_marker, content, block.end_marker);

        if path.exists() {
            let existing = std::fs::read_to_string(path)?;

            if let (Some(start), Some(end)) = (
                existing.find(&block.start_marker),
                existing.find(&block.end_marker),
            ) {
                // Block exists — update in-place
                let end_pos = end + block.end_marker.len();
                let mut new_content = String::with_capacity(existing.len() + 512);
                new_content.push_str(&existing[..start]);
                new_content.push_str(&block_text);
                new_content.push_str(&existing[end_pos..]);
                std::fs::write(path, new_content)?;
                Ok(InjectResult::Updated)
            } else {
                // No block — prepend
                let mut new_content = block_text;
                new_content.push_str("\n\n");
                new_content.push_str(&existing);
                std::fs::write(path, new_content)?;
                Ok(InjectResult::Prepended)
            }
        } else {
            // New file
            std::fs::write(path, &block_text)?;
            Ok(InjectResult::Created)
        }
    }

    /// Check if a file has a managed block.
    pub fn has_block(&self, path: &Path, block_name: &str) -> bool {
        let block = match self.registry.get(block_name) {
            Some(b) => b,
            None => return false,
        };
        if !path.exists() {
            return false;
        }
        match std::fs::read_to_string(path) {
            Ok(content) => {
                content.contains(&block.start_marker) && content.contains(&block.end_marker)
            }
            Err(_) => false,
        }
    }

    /// Read the content of a managed block from a file.
    ///
    /// Returns `None` if the file doesn't exist or has no block.
    pub fn read_block(&self, path: &Path, block_name: &str) -> Option<String> {
        let block = self.registry.get(block_name)?;
        let content = std::fs::read_to_string(path).ok()?;
        let start = content.find(&block.start_marker)?;
        let end = content.find(&block.end_marker)? + block.end_marker.len();
        if start > end {
            return None;
        }
        Some(content[start..end].to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    fn test_registry() -> BlockRegistry {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        reg.register(BlockDef::new("DONT"));
        reg.register(BlockDef::with_markers(
            "ah:managed",
            "<!-- ah:managed:start -->",
            "<!-- ah:managed:end -->",
        ));
        reg
    }

    fn test_injector() -> BlockInjector {
        BlockInjector::new(test_registry())
    }

    // ── BlockDef ──────────────────────────────────────────────────────

    #[test]
    fn test_block_def_new_generates_markers() {
        let b = BlockDef::new("WAI");
        assert_eq!(b.name, "WAI");
        assert_eq!(b.start_marker, "<!-- WAI:START -->");
        assert_eq!(b.end_marker, "<!-- WAI:END -->");
    }

    #[test]
    fn test_block_def_with_custom_markers() {
        let b = BlockDef::with_markers(
            "ah:managed",
            "<!-- ah:managed:start -->",
            "<!-- ah:managed:end -->",
        );
        assert_eq!(b.name, "ah:managed");
        assert_eq!(b.start_marker, "<!-- ah:managed:start -->");
        assert_eq!(b.end_marker, "<!-- ah:managed:end -->");
    }

    // ── BlockRegistry ─────────────────────────────────────────────────

    #[test]
    fn test_registry_empty_by_default() {
        let reg = BlockRegistry::new();
        assert!(reg.names().is_empty());
    }

    #[test]
    fn test_registry_register_and_retrieve() {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        assert!(reg.has("WAI"));
        assert_eq!(reg.get("WAI").unwrap().name, "WAI");
    }

    #[test]
    fn test_registry_replace_existing() {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        reg.register(BlockDef::new("WAI")); // replace
        assert_eq!(reg.names().len(), 1);
    }

    #[test]
    fn test_registry_multiple_blocks() {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        reg.register(BlockDef::new("DONT"));
        reg.register(BlockDef::new("OPENSPEC"));
        assert_eq!(reg.names().len(), 3);
    }

    #[test]
    fn test_registry_get_unknown() {
        let reg = BlockRegistry::new();
        assert!(reg.get("UNKNOWN").is_none());
    }

    // ── BlockInjector: create new file ─────────────────────────────────

    #[test]
    fn test_inject_creates_new_file() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();
        let result = injector.inject(&path, "WAI", "\n# Content\n").unwrap();
        assert_eq!(result, InjectResult::Created);
        assert!(path.exists());
    }

    #[test]
    fn test_inject_creates_file_with_markers() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();
        injector.inject(&path, "WAI", "\n# Content\n").unwrap();
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- WAI:START -->"));
        assert!(content.contains("<!-- WAI:END -->"));
        assert!(content.contains("# Content"));
    }

    // ── BlockInjector: update existing block ───────────────────────────

    #[test]
    fn test_inject_updates_existing_block() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();

        // First injection
        injector.inject(&path, "WAI", "\n# Old content\n").unwrap();

        // Second injection — update
        let result = injector.inject(&path, "WAI", "\n# New content\n").unwrap();
        assert_eq!(result, InjectResult::Updated);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("# New content"));
        assert!(!content.contains("# Old content"));
    }

    #[test]
    fn test_inject_update_does_not_duplicate_markers() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();

        injector.inject(&path, "WAI", "content").unwrap();
        injector.inject(&path, "WAI", "updated").unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let start_count = content.matches("<!-- WAI:START -->").count();
        let end_count = content.matches("<!-- WAI:END -->").count();
        assert_eq!(start_count, 1, "should have exactly one WAI:START");
        assert_eq!(end_count, 1, "should have exactly one WAI:END");
    }

    // ── BlockInjector: prepend to existing file ────────────────────────

    #[test]
    fn test_inject_prepends_to_existing_file() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "# Existing file\n").unwrap();

        let injector = test_injector();
        let result = injector.inject(&path, "WAI", "\n# New block\n").unwrap();
        assert_eq!(result, InjectResult::Prepended);

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("<!-- WAI:START -->"));
        assert!(content.contains("# Existing file"));
    }

    // ── BlockInjector: has_block ───────────────────────────────────────

    #[test]
    fn test_has_block_false_when_file_missing() {
        let dir = tmp();
        let injector = test_injector();
        assert!(!injector.has_block(&dir.path().join("nonexistent.md"), "WAI"));
    }

    #[test]
    fn test_has_block_false_when_no_block() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "# No block\n").unwrap();
        let injector = test_injector();
        assert!(!injector.has_block(&path, "WAI"));
    }

    #[test]
    fn test_has_block_true_when_block_present() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();
        injector.inject(&path, "WAI", "content").unwrap();
        assert!(injector.has_block(&path, "WAI"));
    }

    // ── BlockInjector: read_block ──────────────────────────────────────

    #[test]
    fn test_read_block_returns_none_when_missing() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        std::fs::write(&path, "# No block\n").unwrap();
        let injector = test_injector();
        assert!(injector.read_block(&path, "WAI").is_none());
    }

    #[test]
    fn test_read_block_returns_content() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();
        injector
            .inject(&path, "WAI", "\n# Block content\n")
            .unwrap();
        let content = injector.read_block(&path, "WAI").unwrap();
        assert!(content.contains("<!-- WAI:START -->"));
        assert!(content.contains("<!-- WAI:END -->"));
        assert!(content.contains("# Block content"));
    }

    // ── Custom markers ─────────────────────────────────────────────────

    #[test]
    fn test_custom_markers_work() {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::with_markers(
            "ah:managed",
            "<!-- ah:managed:start -->",
            "<!-- ah:managed:end -->",
        ));
        let injector = BlockInjector::new(reg);

        let dir = tmp();
        let path = dir.path().join("test.md");
        injector
            .inject(&path, "ah:managed", "\n# ah content\n")
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- ah:managed:start -->"));
        assert!(content.contains("<!-- ah:managed:end -->"));
    }

    // ── Error handling ─────────────────────────────────────────────────

    #[test]
    fn test_inject_unknown_block_returns_error() {
        let injector = BlockInjector::new(BlockRegistry::new());
        let dir = tmp();
        let path = dir.path().join("test.md");
        let result = injector.inject(&path, "UNKNOWN", "content");
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_blocks_in_same_file() {
        let dir = tmp();
        let path = dir.path().join("test.md");
        let injector = test_injector();

        injector.inject(&path, "WAI", "\n# WAI content\n").unwrap();
        injector
            .inject(&path, "DONT", "\n# DONT content\n")
            .unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("<!-- WAI:START -->"));
        assert!(content.contains("<!-- WAI:END -->"));
        assert!(content.contains("<!-- DONT:START -->"));
        assert!(content.contains("<!-- DONT:END -->"));
    }

    #[test]
    fn test_reregister_replaces_old_block() {
        let mut reg = BlockRegistry::new();
        reg.register(BlockDef::new("WAI"));
        reg.register(BlockDef::new("WAI"));
        assert_eq!(reg.names().len(), 1);
    }
}
