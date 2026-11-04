//! The shared error taxonomy (`ErrorTaxonomy`) every eval in the suite
//! classifies faults with.

/// Structured classification of an eval-trial failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorTaxonomy {
    /// Tool returned `ok: false` with a populated `hints` vector, but the
    /// agent's subsequent command ignored the suggested fix.
    EnvelopeHintBlindness,
    /// Agent executed an illegal state transition or bypassed state checks
    /// by manually editing local state files.
    StateMachineViolation,
    /// Agent overwrote or deleted AGENTS.md managed-block tags, disrupting
    /// future tool injection.
    ManagedBlockCorruption,
    /// Agent produced text output claiming to have run a command without a
    /// corresponding subprocess call in the terminal environment.
    ToolExecutionHallucination,
    /// Agent encountered an unexpected error but failed to refresh its
    /// project orientation context (e.g. skipped `prime`/`status`).
    ContextRecoveryFailure,
    /// Agent defaulted to a blunt strategy, ignoring the specialized tool
    /// the environment provides (e.g. full test suite instead of test
    /// selection).
    ToolDiscoveryFailure,
    /// Agent acted on stale documentation instead of the tool's structured
    /// output contradicting it (add-aix-eval-loop §3, corpus B171/B128).
    DocDriftBlindness,
    /// Live-tier agent failed to produce a parseable structured action
    /// (`command` + `done`) after one re-ask — the re-ask consumes a turn
    /// (add-evals-guidelines, live action protocol). Distinct from
    /// `ToolExecutionHallucination`, which stays a replay-tier code for
    /// claims of runs that never happened.
    ActionFormatViolation,
}

impl ErrorTaxonomy {
    /// The machine-readable `ERR_*` code.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EnvelopeHintBlindness => "ERR_ENVELOPE_HINT_BLINDNESS",
            Self::StateMachineViolation => "ERR_STATE_MACHINE_VIOLATION",
            Self::ManagedBlockCorruption => "ERR_MANAGED_BLOCK_CORRUPTION",
            Self::ToolExecutionHallucination => "ERR_TOOL_EXECUTION_HALLUCINATION",
            Self::ContextRecoveryFailure => "ERR_CONTEXT_RECOVERY_FAILURE",
            Self::ToolDiscoveryFailure => "ERR_TOOL_DISCOVERY_FAILURE",
            Self::DocDriftBlindness => "ERR_DOC_DRIFT_BLINDNESS",
            Self::ActionFormatViolation => "ERR_ACTION_FORMAT_VIOLATION",
        }
    }

    /// Parse an `ERR_*` code back into a variant.
    pub fn from_code(code: &str) -> Option<Self> {
        Some(match code {
            "ERR_ENVELOPE_HINT_BLINDNESS" => Self::EnvelopeHintBlindness,
            "ERR_STATE_MACHINE_VIOLATION" => Self::StateMachineViolation,
            "ERR_MANAGED_BLOCK_CORRUPTION" => Self::ManagedBlockCorruption,
            "ERR_TOOL_EXECUTION_HALLUCINATION" => Self::ToolExecutionHallucination,
            "ERR_CONTEXT_RECOVERY_FAILURE" => Self::ContextRecoveryFailure,
            "ERR_TOOL_DISCOVERY_FAILURE" => Self::ToolDiscoveryFailure,
            "ERR_DOC_DRIFT_BLINDNESS" => Self::DocDriftBlindness,
            "ERR_ACTION_FORMAT_VIOLATION" => Self::ActionFormatViolation,
            _ => return None,
        })
    }

    /// One-line, agent-actionable description of the failure mode.
    pub fn description(&self) -> &'static str {
        match self {
            Self::EnvelopeHintBlindness => {
                "the tool suggested a fix in its error envelope, but the agent ignored it"
            }
            Self::StateMachineViolation => {
                "the agent made an illegal state transition or bypassed state checks"
            }
            Self::ManagedBlockCorruption => "the agent overwrote or deleted managed-block tags",
            Self::ToolExecutionHallucination => {
                "the agent claimed to run a command without a corresponding subprocess call"
            }
            Self::ContextRecoveryFailure => {
                "the agent hit an error but did not refresh its orientation context"
            }
            Self::ToolDiscoveryFailure => {
                "the agent ignored the specialized tool and used a blunt strategy"
            }
            Self::DocDriftBlindness => {
                "the agent trusted stale documentation over the tool's structured output"
            }
            Self::ActionFormatViolation => {
                "the agent failed to produce a parseable structured action after one re-ask"
            }
        }
    }
}

impl std::fmt::Display for ErrorTaxonomy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.description())
    }
}

// ---------------------------------------------------------------------------
// Envelope assertions
// ---------------------------------------------------------------------------
