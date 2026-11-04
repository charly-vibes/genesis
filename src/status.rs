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

pub use bridge::DoctorStatusBridge;

use crate::envelope::{Envelope, EnvelopeKind, Warning};
use serde::Serialize;
use std::path::Path;

// ── StatusLevel ───────────────────────────────────────────────────────

/// Health level for a status item or section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StatusLevel {
    /// Everything is working as expected.
    Healthy,
    /// Non-blocking concerns detected.
    Warning,
    /// Blocking issues detected.
    Error,
}

impl StatusLevel {
    /// Returns `true` if this is an issue (warning or error).
    pub fn is_issue(&self) -> bool {
        matches!(self, StatusLevel::Warning | StatusLevel::Error)
    }

    /// Returns `true` if this is blocking.
    pub fn is_error(&self) -> bool {
        matches!(self, StatusLevel::Error)
    }
}

// ── StatusItem ────────────────────────────────────────────────────────

/// A single status item in a section.
#[derive(Debug, Clone, Serialize)]
pub struct StatusItem {
    /// Short label (e.g., `"Projects"`, `"Config"`, `"Doctor"`).
    pub label: String,
    /// Human-readable value (e.g., `"3 active"`, `"1 issue found"`).
    pub value: String,
    /// Health level.
    pub level: StatusLevel,
}

impl StatusItem {
    /// Create a healthy status item.
    pub fn healthy(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            level: StatusLevel::Healthy,
        }
    }

    /// Create a warning status item.
    pub fn warning(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            level: StatusLevel::Warning,
        }
    }

    /// Create an error status item.
    pub fn error(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
            level: StatusLevel::Error,
        }
    }
}

// ── StatusSection ─────────────────────────────────────────────────────

/// A status section contributed by one tool.
#[derive(Debug, Clone, Serialize)]
pub struct StatusSection {
    /// Tool name (e.g., `"wai"`, `"dont"`).
    pub tool: String,
    /// Overall level for this section (aggregated from items).
    pub level: StatusLevel,
    /// Summary message (e.g., `"3 checks, 1 warning"`).
    pub summary: String,
    /// Individual status items.
    pub items: Vec<StatusItem>,
    /// Suggested next-step commands.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suggestions: Vec<String>,
}

impl StatusSection {
    /// Create a healthy section with a summary message.
    pub fn healthy(tool: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            tool: tool.into(),
            level: StatusLevel::Healthy,
            summary: summary.into(),
            items: Vec::new(),
            suggestions: Vec::new(),
        }
    }

    /// Create a section with items. The level is auto-derived from items.
    pub fn with_items(
        tool: impl Into<String>,
        summary: impl Into<String>,
        items: Vec<StatusItem>,
    ) -> Self {
        let level = items
            .iter()
            .map(|i| i.level)
            .max_by_key(|l| match l {
                StatusLevel::Error => 2,
                StatusLevel::Warning => 1,
                StatusLevel::Healthy => 0,
            })
            .unwrap_or(StatusLevel::Healthy);
        Self {
            tool: tool.into(),
            level,
            summary: summary.into(),
            items,
            suggestions: Vec::new(),
        }
    }

    /// Add a suggestion (fluent).
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestions.push(suggestion.into());
        self
    }
}

// ── MultiToolStatus ───────────────────────────────────────────────────

/// Aggregated status report from all registered contributors.
#[derive(Debug, Clone, Serialize)]
pub struct MultiToolStatus {
    /// Sections, one per registered tool (in registration order).
    pub sections: Vec<StatusSection>,
}

impl MultiToolStatus {
    /// Compute the aggregate summary across all sections.
    pub fn summary(&self) -> StatusSummary {
        let mut summary = StatusSummary {
            total: self.sections.len(),
            healthy: 0,
            warnings: 0,
            errors: 0,
        };
        for section in &self.sections {
            summary.bump(section.level);
        }
        summary
    }

    /// Whether all sections are healthy.
    pub fn is_healthy(&self) -> bool {
        self.sections
            .iter()
            .all(|s| s.level == StatusLevel::Healthy)
    }

    /// Whether there are any errors.
    pub fn has_errors(&self) -> bool {
        self.sections.iter().any(|s| s.level == StatusLevel::Error)
    }

    /// Collect all suggestions from all sections.
    pub fn all_suggestions(&self) -> Vec<&str> {
        self.sections
            .iter()
            .flat_map(|s| s.suggestions.iter().map(|s| s.as_str()))
            .collect()
    }

    /// Serialize to a JSON envelope.
    ///
    /// `cli_version` must be the caller's own CLI version (see
    /// [`crate::envelope::Envelope::success`]).
    pub fn to_envelope(&self, cli_version: &str) -> Envelope<&Self> {
        let warnings: Vec<Warning> = self
            .sections
            .iter()
            .filter(|s| s.level == StatusLevel::Warning)
            .map(|s| Warning {
                rule_name: s.tool.clone(),
                entity_id: None,
                message: s.summary.clone(),
                suggested_remediation: s.suggestions.first().cloned(),
            })
            .collect();

        Envelope::success(cli_version, EnvelopeKind::Ok, self, warnings, vec![])
    }
}

// ── StatusSummary ─────────────────────────────────────────────────────

/// Aggregate health summary.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct StatusSummary {
    /// Total registered tools.
    pub total: usize,
    /// Tools reporting healthy.
    pub healthy: usize,
    /// Tools reporting warnings.
    pub warnings: usize,
    /// Tools reporting errors.
    pub errors: usize,
}

impl StatusSummary {
    /// Tally one section's level into the aggregate counts.
    fn bump(&mut self, level: StatusLevel) {
        match level {
            StatusLevel::Healthy => self.healthy += 1,
            StatusLevel::Warning => self.warnings += 1,
            StatusLevel::Error => self.errors += 1,
        }
    }

    /// Whether all tools are healthy.
    pub fn is_healthy(&self) -> bool {
        self.errors == 0 && self.warnings == 0
    }
}

// ── StatusContributor trait ───────────────────────────────────────────

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

// ── StatusBuilder ─────────────────────────────────────────────────────

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

// ── Tests ─────────────────────────────────────────────────────────────

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
