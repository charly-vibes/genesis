//! Doctor report types: entries, summary, and the serializable report.

use crate::envelope::{Envelope, EnvelopeKind};
use serde::Serialize;

// ── CheckStatus ───────────────────────────────────────────────────────

/// Outcome status for a single doctor check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    /// Check passed — no issues found.
    Pass,
    /// Check found a non-blocking concern.
    Warn,
    /// Check found a blocking issue.
    Fail,
}

impl CheckStatus {
    /// Returns `true` if this is a passing status.
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckStatus::Pass)
    }

    /// Returns `true` if this is a warning or failure.
    pub fn is_issue(&self) -> bool {
        matches!(self, CheckStatus::Warn | CheckStatus::Fail)
    }

    /// Returns `true` if this is a failure (blocking).
    pub fn is_fail(&self) -> bool {
        matches!(self, CheckStatus::Fail)
    }
}

// ── CheckEntry ────────────────────────────────────────────────────────

/// A single check's result in the doctor report.
#[derive(Debug, Clone, Serialize)]
pub struct CheckEntry {
    /// Unique check name (e.g., `"wai.directory-structure"`).
    pub name: String,
    /// Human-readable description of what this check validates.
    pub description: String,
    /// The outcome status.
    pub status: CheckStatus,
    /// Human-readable message describing the result.
    pub message: String,
    /// Optional CLI command to fix the issue (e.g., `"wai init"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix: Option<String>,
}

impl CheckEntry {
    /// Create a passing check entry.
    pub fn pass(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            fix: None,
        }
    }

    /// Create a warning check entry.
    pub fn warn(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            fix: None,
        }
    }

    /// Create a failing check entry with an optional fix command.
    pub fn fail(
        name: impl Into<String>,
        description: impl Into<String>,
        message: impl Into<String>,
        fix: Option<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            fix,
        }
    }
}

// ── DoctorSummary ─────────────────────────────────────────────────────

/// Summary counts for a doctor run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct DoctorSummary {
    /// Number of passing checks.
    pub pass: usize,
    /// Number of warning checks.
    pub warn: usize,
    /// Number of failing checks.
    pub fail: usize,
}

impl DoctorSummary {
    /// Create a summary from an iterator of check entries.
    pub fn from_checks<'a>(checks: impl IntoIterator<Item = &'a CheckEntry>) -> Self {
        let checks: Vec<&CheckEntry> = checks.into_iter().collect();
        let count = |status: CheckStatus| {
            checks
                .iter()
                .filter(|c| c.status == status)
                .count()
        };
        Self {
            pass: count(CheckStatus::Pass),
            warn: count(CheckStatus::Warn),
            fail: count(CheckStatus::Fail),
        }
    }

    /// Returns `true` if all checks passed (no warnings or failures).
    pub fn is_healthy(&self) -> bool {
        self.warn == 0 && self.fail == 0
    }

    /// Returns `true` if there are any failures (blocking).
    pub fn has_failures(&self) -> bool {
        self.fail > 0
    }

    /// Returns `true` if there are any issues (warnings or failures).
    pub fn has_issues(&self) -> bool {
        self.warn > 0 || self.fail > 0
    }
}

// ── DoctorReport ──────────────────────────────────────────────────────

/// Full diagnostic report from a doctor run.
#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    /// Tool name that produced this report (e.g., `"wai"`).
    pub tool: String,
    /// All check results.
    pub checks: Vec<CheckEntry>,
    /// Aggregate summary.
    pub summary: DoctorSummary,
}

impl DoctorReport {
    /// Create a new doctor report.
    pub fn new(tool: impl Into<String>, checks: Vec<CheckEntry>) -> Self {
        let summary = DoctorSummary::from_checks(&checks);
        Self {
            tool: tool.into(),
            checks,
            summary,
        }
    }

    /// Returns `true` if all checks passed.
    pub fn is_healthy(&self) -> bool {
        self.summary.is_healthy()
    }

    /// Returns the exit code for this report (0 = healthy, 1 = failures).
    pub fn exit_code(&self) -> i32 {
        if self.summary.has_failures() { 1 } else { 0 }
    }

    /// Serialize to a JSON envelope with kind `Doctor`.
    ///
    /// `cli_version` must be the caller's own CLI version (see
    /// [`crate::envelope::Envelope::success`]).
    pub fn to_envelope(&self, cli_version: &str) -> Envelope<&Self> {
        let hints = self
            .checks
            .iter()
            .filter_map(|c| {
                c.fix.as_ref().map(|fix| crate::envelope::HintEntry {
                    command: fix.clone(),
                    description: c.message.clone(),
                })
            })
            .collect::<Vec<_>>();

        let warnings = self
            .checks
            .iter()
            .filter(|c| matches!(c.status, CheckStatus::Warn))
            .map(|c| crate::envelope::Warning {
                rule_name: c.name.clone(),
                entity_id: None,
                message: c.message.clone(),
                suggested_remediation: c.fix.clone(),
            })
            .collect();

        Envelope::success(cli_version, EnvelopeKind::Doctor, self, warnings, hints)
    }
}
