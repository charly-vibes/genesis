//! Core data types for the status dashboard: [`StatusLevel`], [`StatusItem`],
//! [`StatusSection`], the aggregated [`MultiToolStatus`], and [`StatusSummary`].

use crate::envelope::{Envelope, EnvelopeKind, Warning};
use serde::Serialize;

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

/// Highest severity among `items`; [`StatusLevel::Healthy`] when empty.
fn items_level(items: &[StatusItem]) -> StatusLevel {
    items
        .iter()
        .map(|i| i.level)
        .max_by_key(|l| match l {
            StatusLevel::Error => 2,
            StatusLevel::Warning => 1,
            StatusLevel::Healthy => 0,
        })
        .unwrap_or(StatusLevel::Healthy)
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
        Self {
            tool: tool.into(),
            level: items_level(&items),
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
