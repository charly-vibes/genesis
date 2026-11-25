//! Notice and suggestion payload types carried on envelopes: structured
//! warnings, remediation entries, unmet clauses, and next-step hints.

use serde::{Deserialize, Serialize};

/// A structured warning associated with a rule or check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Warning {
    pub rule_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_remediation: Option<String>,
}

/// A suggested command the user can run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationEntry {
    pub command: String,
    pub description: String,
}

/// An unmet clause within an error result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnmetClause {
    pub clause: String,
    pub fix: String,
}

/// A hint entry — a suggested next command for the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HintEntry {
    pub command: String,
    pub description: String,
}
