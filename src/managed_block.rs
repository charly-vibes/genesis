//! Managed block injector (`<!-- …:START -->`/`<!-- …:END -->`).
//!
//! A block is a delimited region in a Markdown file, owned by one tool. The
//! injector resolves a [`BlockDef`] by name in its [`BlockRegistry`], matches
//! that block's markers literally, and splices only that span; text outside
//! it — including other tools' blocks — is copied through unchanged. A file
//! without the block gets it prepended. Marker dialects that are not
//! `<!-- NAME:START -->` are declared with [`BlockDef::with_markers`].
//!
//! The ecosystem contract (ownership, retirement, dialects, drift, facts vs.
//! procedures) is documented in
//! `docs/src/explanation/agent-instruction-boundaries.md`; this module owns
//! the API, not the fleet policy.
//!
//! ## Provenance footer
//!
//! Injection optionally appends a provenance footer line inside the end
//! marker: generator name, generator version, registry block name, and a
//! short content hash. The hash covers only the injected content (never the
//! footer itself), so drift is machine-checkable by recomputing expected
//! content and comparing hashes. The hash is FNV-1a 64-bit truncated to 8
//! hex digits — a pinned, rustc-stable algorithm, unlike the
//! `std::hash::DefaultHasher` used before it. Footer is opt-in via
//! [`BlockInjector::with_provenance`]; footer-less output is byte-identical
//! to pre-footer releases. See [`content_sha8`], [`provenance_footer`], and
//! [`split_provenance_footer`].
//!
//! Ported from `wai/src/managed_block.rs`. [`BlockDef`] replaces wai's
//! hardcoded marker constants and [`BlockInjector`] replaces its inject
//! function; wai-specific content generation stayed out.

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
    ///
    /// # Panics
    ///
    /// Panics if the block's start or end marker is already used by a
    /// registered block under a different name. Shared markers make both
    /// blocks match each other's regions — silent cross-block capture — so
    /// the conflict is rejected at registration instead.
    pub fn register(&mut self, block: BlockDef) {
        self.blocks.retain(|b| b.name != block.name);
        assert!(
            !self
                .blocks
                .iter()
                .any(|b| b.start_marker == block.start_marker || b.end_marker == block.end_marker),
            "block '{}' shares a marker with an already-registered block",
            block.name
        );
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

mod provenance;
mod splice;

pub use provenance::{content_sha8, parse_footer_sha, provenance_footer, split_provenance_footer};

use splice::{block_span, prepend_block, splice_block, unknown_block_error};

/// Injector for managed blocks.
///
/// Reads, writes, and updates managed blocks in files.
pub struct BlockInjector {
    registry: BlockRegistry,
    /// Provenance stamp `(generator, version)` — `None` renders footer-less.
    ///
    /// Both values are supplied by the caller: the generator's identity is
    /// the *injecting* tool's, which this crate cannot know.
    provenance: Option<(String, String)>,
}

impl BlockInjector {
    /// Create a new injector with the given block registry.
    pub fn new(registry: BlockRegistry) -> Self {
        Self {
            registry,
            provenance: None,
        }
    }

    /// Enable the provenance footer with the given generator name and
    /// version.
    ///
    /// The caller stamps its own identity — this crate cannot know which
    /// tool is injecting, so the version must be passed explicitly rather
    /// than borrowed from genesis's `CARGO_PKG_VERSION`. Footer-less
    /// injectors (the default) produce byte-identical output to pre-footer
    /// releases.
    pub fn with_provenance(mut self, generator: &str, version: &str) -> Self {
        self.provenance = Some((generator.to_string(), version.to_string()));
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
        let block = self
            .registry
            .get(block_name)
            .ok_or_else(|| unknown_block_error(block_name))?;
        let block_text = self.render_block(block, content);
        if path.exists() {
            self.inject_into_existing(path, block, &block_text)
        } else {
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
    /// Compose the managed-block text, with provenance when configured.
    fn render_block(&self, block: &BlockDef, content: &str) -> String {
        match &self.provenance {
            Some((generator, version)) => format!(
                "{}{}\n{}\n{}",
                block.start_marker,
                content,
                provenance_footer(generator, version, &block.name, content),
                block.end_marker
            ),
            None => format!("{}{}{}", block.start_marker, content, block.end_marker),
        }
    }

    /// Update an existing file: splice the block in place, or prepend it.
    fn inject_into_existing(
        &self,
        path: &Path,
        block: &BlockDef,
        block_text: &str,
    ) -> Result<InjectResult, std::io::Error> {
        let existing = std::fs::read_to_string(path)?;
        if let Some((start, end_pos)) = block_span(&existing, block) {
            let new_content = splice_block(&existing, start, end_pos, block_text);
            std::fs::write(path, new_content)?;
            Ok(InjectResult::Updated)
        } else {
            prepend_block(path, &existing, block_text)?;
            Ok(InjectResult::Prepended)
        }
    }
}
#[cfg(test)]
mod tests;
