//! Cross-tool status/prime dashboard.
//!
//! Provides a [`StatusContributor`] trait that tools implement to register
//! their health state, and a [`StatusBuilder`] that aggregates all registered
//! contributors into a unified status report.
//!
//! Pairs with the [`doctor`] module: doctor finds issues, status surfaces
//! them in a cross-tool dashboard.
//!
//! ## Usage
//!
//! ```rust
//! use genesis::status::{StatusContributor, StatusBuilder, StatusLevel, StatusSection};
//! use std::path::Path;
//!
//! struct MyToolStatus;
//! impl StatusContributor for MyToolStatus {
//!     fn name(&self) -> &'static str { "my-tool" }
//!     fn status(&self, _repo: &Path) -> Result<StatusSection, String> {
//!         Ok(StatusSection::healthy("my-tool", "all systems go"))
//!     }
//! }
//!
//! let mut builder = StatusBuilder::new();
//! builder.register(Box::new(MyToolStatus));
//! let report = builder.build(&std::env::current_dir().unwrap()).unwrap();
//! println!("tools: {} healthy: {}", report.sections.len(), report.summary().is_healthy());
//! ```

mod bridge;
mod model;

pub use bridge::DoctorStatusBridge;
pub use model::{MultiToolStatus, StatusItem, StatusLevel, StatusSection, StatusSummary};

use std::path::Path;

/// A tool that can report its health status.
///
/// Implement this trait for each tool that wants to appear in the
/// cross-tool status dashboard.
pub trait StatusContributor: Send + Sync {
    /// Unique tool name (e.g., `"wai"`).
    fn name(&self) -> &'static str;

    /// Produce a status section for the current project root.
    ///
    /// Return `Err` if the tool is not initialized or unreachable.
    fn status(&self, repo_root: &Path) -> Result<StatusSection, String>;
}

/// Aggregates status from multiple [`StatusContributor`]s.
///
/// # Example
///
/// ```rust
/// use genesis::status::{StatusBuilder, StatusContributor, StatusSection};
/// use std::path::Path;
///
/// let mut builder = StatusBuilder::new();
/// builder.register(Box::new(DummyStatus));
/// let report = builder.build(Path::new("/tmp")).unwrap();
/// assert_eq!(report.sections.len(), 1);
///
/// struct DummyStatus;
/// impl StatusContributor for DummyStatus {
///     fn name(&self) -> &'static str { "dummy" }
///     fn status(&self, _: &Path) -> Result<StatusSection, String> {
///         Ok(StatusSection::healthy("dummy", "ok"))
///     }
/// }
/// ```
pub struct StatusBuilder {
    contributors: Vec<Box<dyn StatusContributor>>,
}

impl StatusBuilder {
    /// Create an empty status builder.
    pub fn new() -> Self {
        Self {
            contributors: Vec::new(),
        }
    }

    /// Register a status contributor.
    pub fn register(&mut self, contributor: Box<dyn StatusContributor>) {
        self.contributors.push(contributor);
    }

    /// Convenience: register a tool's doctor as a status contributor.
    ///
    /// Equivalent to `builder.register(Box::new(DoctorStatusBridge::new(name, runner)))`.
    pub fn register_doctor(&mut self, name: &'static str, runner: crate::doctor::DoctorRunner) {
        self.contributors
            .push(Box::new(DoctorStatusBridge::new(name, runner)));
    }

    /// Build the aggregated status report.
    ///
    /// Contributors that return `Err` are included as error sections.
    pub fn build(&self, repo_root: &Path) -> Result<MultiToolStatus, String> {
        let sections = self
            .contributors
            .iter()
            .map(|contributor| contributor_section(contributor.as_ref(), repo_root))
            .collect();

        Ok(MultiToolStatus { sections })
    }

    /// Number of registered contributors.
    pub fn len(&self) -> usize {
        self.contributors.len()
    }

    /// Whether no contributors are registered.
    pub fn is_empty(&self) -> bool {
        self.contributors.is_empty()
    }
}

impl Default for StatusBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "status/tests.rs"]
mod tests;

#[cfg(test)]
#[path = "status/tests_builder.rs"]
mod tests_builder;

/// One contributor's section; failures become error sections.
fn contributor_section(contributor: &dyn StatusContributor, repo_root: &Path) -> StatusSection {
    contributor
        .status(repo_root)
        .unwrap_or_else(|e| StatusSection {
            tool: contributor.name().to_string(),
            level: StatusLevel::Error,
            summary: format!("unreachable: {e}"),
            items: vec![],
            suggestions: vec![format!("{} init", contributor.name())],
        })
}
