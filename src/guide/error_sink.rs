//! `ErrorSink`: the error path (tool name, suggestion footer, feedback
//! fallback, best-effort scratch recording).

use std::io::Write;

use super::verbosity::Verbosity;
use crate::feedback::scratch as scratch_mod;
use crate::suggestions::Suggestion;

// ---------------------------------------------------------------------------
// ErrorSink
// ---------------------------------------------------------------------------

/// Configuration for self-healing error handling.
///
/// Wires together error printing, scratch persistence, and optional
/// feedback fallback.
#[derive(Debug, Clone)]
pub struct ErrorSink {
    /// The tool name (used for scratch directory and feedback subcommand).
    pub tool_name: String,
    /// Persist the last error to the error scratch.
    pub scratch: bool,
    /// Print the error with a Suggestion::Fix footer.
    pub suggest: bool,
    /// Include the full ContextBundle in the error output.
    pub context: bool,
    /// Name of the feedback subcommand, if the tool has one.
    /// If None, no feedback suggestion is printed.
    pub feedback_subcommand: Option<String>,
    /// The current verbosity level.
    pub verbosity: Verbosity,
}

impl ErrorSink {
    /// Create a new error sink with sensible defaults.
    pub fn new(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            scratch: true,
            suggest: true,
            context: false,
            feedback_subcommand: Some("feedback".into()),
            verbosity: Verbosity::Normal,
        }
    }

    /// Handle an error — print it, optionally write to scratch, and
    /// optionally suggest a feedback command.
    pub fn handle(&self, err: &dyn std::error::Error, stderr: &mut impl Write) {
        self.handle_message(&err.to_string(), None, stderr);
    }

    /// Handle an error with an explicit suggestion footer override.
    pub fn handle_with_footer(
        &self,
        err: &dyn std::error::Error,
        suggestion: &Suggestion,
        stderr: &mut impl Write,
    ) {
        self.handle_message(&err.to_string(), Some(suggestion), stderr);
    }

    /// Core error handler implementation.
    fn handle_message(
        &self,
        message: &str,
        suggestion: Option<&Suggestion>,
        stderr: &mut impl Write,
    ) {
        let _ = writeln!(stderr, "{}: {}", self.tool_name, message);
        self.print_footer(suggestion, stderr);
        self.print_feedback_hint(suggestion, stderr);
        self.record_scratch(message, suggestion);
    }

    /// Feedback fallback: printed only when there is no suggestion at all
    /// (no explicit suggestion and the generic footer is disabled).
    fn print_feedback_hint(&self, suggestion: Option<&Suggestion>, stderr: &mut impl Write) {
        if self.feedback_subcommand.is_some() && suggestion.is_none() && !self.suggest {
            let _ = writeln!(
                stderr,
                "Feedback: {} {} bug --from-last-error",
                self.tool_name,
                self.feedback_subcommand.as_deref().unwrap_or("feedback"),
            );
        }
    }

    /// Print the suggestion footer or the generic doctor footer.
    fn print_footer(&self, suggestion: Option<&Suggestion>, stderr: &mut impl Write) {
        if let Some(s) = suggestion {
            if let Some(footer) = s.footer() {
                let _ = writeln!(stderr, "{}", footer);
            }
        } else if self.suggest {
            // No explicit suggestion — print generic footer.
            let _ = writeln!(stderr, "-> Run: {} doctor", self.tool_name);
        }
    }

    /// Write an error record to the scratch file when scratch is enabled.
    fn record_scratch(&self, message: &str, suggestion: Option<&Suggestion>) {
        if !self.scratch {
            return;
        }
        let record = scratch_mod::ErrorRecord {
            ts: chrono_now(),
            argv: vec![self.tool_name.clone()],
            exit: 1,
            footer: suggestion.and_then(|s| s.footer()),
            kind: "error".into(),
            message: Some(message.to_string()),
        };
        scratch_mod::write_scratch_best_effort(&self.tool_name, &record);
    }

    /// Set the verbosity level (fluent).
    pub fn with_verbosity(mut self, verbosity: Verbosity) -> Self {
        self.verbosity = verbosity;
        self
    }

    /// Enable or disable scratch persistence (fluent).
    pub fn with_scratch(mut self, enabled: bool) -> Self {
        self.scratch = enabled;
        self
    }

    /// Enable or disable suggestion footer (fluent).
    pub fn with_suggest(mut self, enabled: bool) -> Self {
        self.suggest = enabled;
        self
    }

    /// Set the feedback subcommand name (fluent).
    pub fn with_feedback(mut self, name: Option<&str>) -> Self {
        self.feedback_subcommand = name.map(|s| s.to_string());
        self
    }
}

/// Get an ISO 8601 timestamp without pulling in chrono.
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs();
    let time_secs = secs % 86400;
    let hours = time_secs / 3600;
    let minutes = (time_secs % 3600) / 60;
    let seconds = time_secs % 60;

    let (y, m, d) = civil_date(secs / 86400);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y, m, d, hours, minutes, seconds
    )
}

/// `(year, month, day)` from whole days since 1970-01-01 (exact for
/// post-2000 dates; simple algorithm, no external dependency).
fn civil_date(days: u64) -> (i64, u32, u32) {
    let (y, remaining) = civil_year(days as i64);
    let (m, d) = civil_month(y, remaining);
    (y, m, d)
}

/// Advance past whole years from 1970 until `days` fits inside year `y`;
/// returns `(y, days left inside y)`.
fn civil_year(days: i64) -> (i64, i64) {
    let mut y = 1970i64;
    let mut remaining = days;
    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    (y, remaining)
}

/// `(month, day)` for `remaining` whole days since January 1 of year `y`.
fn civil_month(y: i64, remaining: i64) -> (u32, u32) {
    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut m = 1u32;
    let mut rem = remaining;
    for &md in &month_days {
        if rem < md {
            break;
        }
        rem -= md;
        m += 1;
    }
    (m, (rem + 1) as u32)
}

fn is_leap(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
