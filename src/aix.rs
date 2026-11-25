//! AIX artifact generation (`llms.txt`/`llm.txt`/agent blocks).
//!
//! Provenance footers: see [`crate::managed_block`] for the injected-block
//! form; this module appends txt-style `# provenance:` lines via the
//! `*_with_provenance` / `*_timestamped` variants (footer-free defaults stay
//! byte-identical to pre-footer output).
//!
//! Provides structured data types and generation functions for the
//! standardized `llms.txt` (concise project summary) and `llm.txt`
//! (detailed project context) files used by the charly-vibes suite.
//!
//! Tools compose their `llm.txt` from [`LlmSection`] values, using the
//! helper functions for common section types (modules, commands, genesis
//! adoption, authorship, links), then fill in tool-specific content.
//!
//! ## Provenance footer
//!
//! Generated artifacts optionally end with a `# provenance:` comment line
//! (a comment in every consumed format) carrying the generator version, an
//! 8-hex content hash, and — in the `*_timestamped` variants — a
//! caller-supplied RFC 3339 generation time. The hash covers only the
//! artifact content, never the footer line itself, so version bumps do not
//! move it and drift checks can recompute-and-compare. Default variants are
//! unchanged and fully deterministic (design D1: determinism beats default
//! timestamps; audit trails opt into `*_timestamped`).
//!
//! ## Self-hosting
//!
//! Genesis generates its own `llms.txt` and `llm.txt` via this module.
//! Run `just aix-gen` to regenerate them from the current module metadata.
//! See `justfile` in the repo root for the regeneration command.

mod generate;
mod meta;
mod sections;

#[cfg(test)]
#[path = "aix/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "aix/tests_provenance.rs"]
mod tests_provenance;

#[cfg(test)]
#[path = "aix/tests_sections.rs"]
mod tests_sections;

pub use generate::{
    generate_llm_txt, generate_llm_txt_bounded, generate_llm_txt_timestamped,
    generate_llm_txt_with_provenance, generate_llms_txt, generate_llms_txt_bounded,
    generate_llms_txt_bounded_with_provenance, generate_llms_txt_timestamped,
    generate_llms_txt_with_provenance, write_llm_txt, write_llms_txt,
};
pub use meta::{ModuleEntry, ProjectMeta, TOKEN_COST_HEURISTIC, TokenCost, estimate_token_cost};
pub use sections::{
    LlmSection, agents_block, authorship_section, commands_section, genesis_adoption_section,
    links_section, modules_section,
};
