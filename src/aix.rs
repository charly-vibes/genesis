//! AIX artifact generation (`llms.txt`/`llm.txt`/agent blocks).
//!
//! Port from wai's AIX generation.
//!
//! ## Bootstrap note
//!
//! Until the `aix` module is stable, genesis's own AIX artifacts are
//! hand-written (see `llms.txt`, `llm.txt`, `AGENTS.md` in the repo root).
//! Once stable, this module will generate them automatically.
//!
//! ## agents_block helper
//!
//! For managed-block injection, use the `managed_block` module instead:
//!
//! ```rust,ignore
//! use genesis::managed_block::{BlockDef, BlockRegistry, BlockInjector};
//!
//! let mut reg = BlockRegistry::new();
//! reg.register(BlockDef::new("MY_TOOL"));
//! let injector = BlockInjector::new(reg);
//! injector.inject(&path, "MY_TOOL", "\n# Content\n");
//! ```
//!
//! TODO: implement full AIX generation once design is finalized.

/// Generate a managed block for an agent instructions file.
///
/// This is a convenience wrapper around `managed_block::BlockInjector`.
/// It creates a block with the standard `<!-- NAME:START -->`/`<!-- NAME:END -->`
/// markers.
pub fn agents_block(name: &str, body: &str) -> String {
    format!("<!-- {}:START -->\n{}\n<!-- {}:END -->", name, body, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agents_block_generates_markers() {
        let block = agents_block("WAI", "# Content\n");
        assert!(block.contains("<!-- WAI:START -->"));
        assert!(block.contains("<!-- WAI:END -->"));
        assert!(block.contains("# Content"));
    }

    #[test]
    fn test_agents_block_start_before_end() {
        let block = agents_block("TEST", "body");
        let start = block.find("<!-- TEST:START -->").unwrap();
        let end = block.find("<!-- TEST:END -->").unwrap();
        assert!(start < end, "START marker must precede END marker");
    }

    #[test]
    fn test_agents_block_roundtrip() {
        let body = "## Tool\n\nRun `tool check`.\n";
        let block = agents_block("DONT", body);
        assert!(block.contains(body));
    }
}
