//! Agent issue reporting (feedback).
//!
//! New module — see `agent-issue-reporting.md` for design.
//!
//! ## Modules
//!
//! - `redactor` — privacy redaction for issue bodies (§5)
//! - `context` — environment context bundle (§3)
//! - `scratch` — error-scratch JSONL writer (§4)
//! - `gh` — GitHub CLI invocation with fallback ladder (§7)

pub mod context;
pub mod gh;
pub mod redactor;
pub mod scratch;
