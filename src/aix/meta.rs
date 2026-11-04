//! Data types shared by the AIX generation submodules.

#[derive(Debug, Clone)]
pub struct ProjectMeta {
    /// Project name (e.g., `"genesis-vibes"`).
    pub name: String,
    /// Short tagline/one-liner.
    pub tagline: String,
    /// Optional repository URL.
    pub repository: Option<String>,
    /// Optional documentation URL.
    pub documentation: Option<String>,
    /// Optional crates.io URL.
    pub crates_io: Option<String>,
}

impl ProjectMeta {
    /// Create a new project metadata with the minimal fields.
    pub fn new(name: &str, tagline: &str) -> Self {
        Self {
            name: name.to_string(),
            tagline: tagline.to_string(),
            repository: None,
            documentation: None,
            crates_io: None,
        }
    }

    /// Builder-style: set the repository URL.
    pub fn with_repository(mut self, url: &str) -> Self {
        self.repository = Some(url.to_string());
        self
    }

    /// Builder-style: set the documentation URL.
    pub fn with_documentation(mut self, url: &str) -> Self {
        self.documentation = Some(url.to_string());
        self
    }

    /// Builder-style: set the crates.io URL.
    pub fn with_crates_io(mut self, url: &str) -> Self {
        self.crates_io = Some(url.to_string());
        self
    }
}

/// A module entry for listing in AIX artifacts.
#[derive(Debug, Clone)]
pub struct ModuleEntry {
    /// Module name (e.g., `"envelope"`).
    pub name: String,
    /// One-line description of the module.
    pub description: String,
}

impl ModuleEntry {
    /// Create a new module entry.
    pub fn new(name: &str, description: &str) -> Self {
        Self {
            name: name.to_string(),
            description: description.to_string(),
        }
    }
}

/// Name of the token-count heuristic shipped with every estimate, so an
/// approximation is never mistaken for an actual count (design D2). The
/// chars/4 heuristic carries a documented **±25% error band** against typical
/// English prose in cl100k-class vocabularies — good enough for budget
/// arbitration, not for billing.
pub const TOKEN_COST_HEURISTIC: &str = "chars/4";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCost {
    /// Estimated token count.
    pub estimate: usize,
    /// Name of the heuristic that produced the estimate.
    pub heuristic: &'static str,
}

/// Estimate the token cost of any generated artifact via the chars/4
/// heuristic (ceiling division), labeled with the heuristic name.
pub fn estimate_token_cost(s: &str) -> TokenCost {
    TokenCost {
        estimate: s.chars().count().div_ceil(4),
        heuristic: TOKEN_COST_HEURISTIC,
    }
}
