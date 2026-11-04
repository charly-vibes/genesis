//! Managed block injector (`<!-- …:START -->`/`<!-- …:END -->`).
//!
//! Port from `wai/src/managed_block.rs` (also used in dont/espectacular).
//!
//! ## Provenance footer
//!
//! Injection optionally appends a provenance footer line inside the end
//! marker: generator name, generator version, registry block name, and a
//! short content hash. The hash covers only the injected content (never the
//! footer itself), so drift is machine-checkable by recomputing expected
//! content and comparing hashes (`ManagedBlockDrift`, suite_linter). Footer
//! is opt-in via [`BlockInjector::with_provenance`]; footer-less output is
//! byte-identical to pre-footer releases.
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

/// Compute the 8-hex-char content hash used in provenance footers.
///
/// Hashing style matches feedback's `repro_hash` (DefaultHasher over the
/// content bytes) so there is one hashing story across the crate. Truncated
/// to 8 hex chars — collision-resistant enough for drift detection.
pub fn content_sha8(content: &str) -> String {
    use std::hash::BuildHasher;
    let full = format!(
        "{:016x}",
        std::hash::BuildHasherDefault::<std::hash::DefaultHasher>::default().hash_one(content)
    );
    full[..8].to_string()
}

/// Build the opt-in provenance footer line for a managed block.
///
/// The footer is an HTML comment (inert in rendered markdown and ignored by
/// plain-text readers) placed inside the block's markers, after the content.
/// The `sha` field covers only `content` — never the footer line itself — so
/// changing the generator version does not move the hash.
pub fn provenance_footer(
    generator: &str,
    version: &str,
    block_name: &str,
    content: &str,
) -> String {
    format!(
        "<!-- provenance: generator={} version={} source={} sha={} -->",
        generator,
        version,
        block_name,
        content_sha8(content)
    )
}

/// Injector for managed blocks.
///
/// Reads, writes, and updates managed blocks in files.
pub struct BlockInjector {
    registry: BlockRegistry,
    provenance_generator: Option<String>,
}

impl BlockInjector {
    /// Create a new injector with the given block registry.
    pub fn new(registry: BlockRegistry) -> Self {
        Self {
            registry,
            provenance_generator: None,
        }
    }

    /// Enable the provenance footer with the given generator name.
    ///
    /// The version is taken from this crate's `CARGO_PKG_VERSION`. Footer-less
    /// injectors (the default) produce byte-identical output to pre-footer
    /// releases.
    pub fn with_provenance(mut self, generator: &str) -> Self {
        self.provenance_generator = Some(generator.to_string());
        self
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

        let block_text = match &self.provenance_generator {
            Some(generator) => format!(
                "{}{}\n{}\n{}",
                block.start_marker,
                content,
                provenance_footer(generator, env!("CARGO_PKG_VERSION"), &block.name, content),
                block.end_marker
            ),
            None => format!("{}{}{}", block.start_marker, content, block.end_marker),
        };

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
#[cfg(test)]
#[path = "managed_block/tests.rs"]
mod tests;
