//! Self-healing error suggestions.
//!
//! Port from `wai/src/suggestions.rs`.
//!
//! ## Changes from wai
//!
//! - Added `footer()` helper on `Suggestion` for "→ Run: …" formatting (task 3.2).
//! - Added `CommandRegistry` for tools to register valid commands (task 3.3).
//! - `SuggestionEngine` now takes a `&CommandRegistry` instead of a slice.
//! - Removed `suggest_context` (directory-specific; not general enough for genesis).
//! - `Suggestion` derives `Serialize` for envelope compatibility.

use serde::Serialize;
use std::collections::HashMap;

/// A registry of valid commands for typo detection.
///
/// Tools register their valid commands once at startup, then pass the
/// registry to `SuggestionEngine` for automatic typo detection.
#[derive(Debug, Clone, Default)]
pub struct CommandRegistry {
    /// Map of tool name -> list of valid commands
    commands: HashMap<String, Vec<String>>,
    /// Flat list of all valid commands across all tools
    flat: Vec<String>,
}

impl CommandRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool's valid commands.
    ///
    /// Subsequent calls for the same tool replace the previous list.
    pub fn register(&mut self, tool: &str, commands: Vec<String>) {
        if let Some(old) = self.commands.remove(tool) {
            self.flat.retain(|c| !old.contains(c));
        }
        self.flat.extend(commands.iter().cloned());
        self.commands.insert(tool.to_string(), commands);
    }

    /// Get all registered commands as string slices.
    pub fn all(&self) -> Vec<&str> {
        self.flat.iter().map(|s| s.as_str()).collect()
    }

    /// Get commands for a specific tool.
    pub fn for_tool(&self, tool: &str) -> Vec<&str> {
        self.commands
            .get(tool)
            .map(|cmds| cmds.iter().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

/// Suggestion types that can be generated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Suggestion {
    /// Typo suggestion with the corrected command.
    DidYouMean {
        original: String,
        suggestion: String,
    },
    /// Wrong order detection (e.g., "project new" -> "new project").
    WrongOrder { original: String, correct: String },
    /// Context hint (e.g., "try running from project root").
    ContextHint {
        message: String,
        path: Option<String>,
    },
    /// Generic fix suggestion.
    Fix {
        description: String,
        command: Option<String>,
    },
}

impl Suggestion {
    /// Format the suggestion as a human-readable message.
    pub fn message(&self) -> String {
        match self {
            Suggestion::Fix {
                description,
                command,
            } => fix_message(description, command.as_deref()),
            other => other.guidance(),
        }
    }

    /// Create a fix suggestion from a hint string.
    ///
    /// The hint is used as both the description and the command suggestion.
    pub fn fix(hint: impl Into<String>) -> Self {
        let s: String = hint.into();
        parse_fix_hint(&s).unwrap_or_else(|| Suggestion::Fix {
            command: Some(s.clone()),
            description: s,
        })
    }

    /// The bare runnable command this suggestion points at, without the
    /// human `→ Run: ` prefix (genesis-r13: JSON `HintEntry.command` must
    /// be executable, not display-formatted).
    pub fn command(&self) -> Option<&str> {
        match self {
            Suggestion::DidYouMean { suggestion, .. } => Some(suggestion),
            Suggestion::WrongOrder { correct, .. } => Some(correct),
            Suggestion::Fix { command, .. } => command.as_deref(),
            Suggestion::ContextHint { .. } => None,
        }
    }

    /// The human guidance text for this suggestion, without the
    /// `→ Run: {command}` footer line (genesis-r13: the JSON hint's
    /// `description` pairs with the bare `command`; the footer is the
    /// human rendering and must not leak into either field).
    pub fn guidance(&self) -> String {
        match self {
            Suggestion::DidYouMean {
                original,
                suggestion,
            } => miscommand_text("Unknown", original, suggestion),
            Suggestion::WrongOrder { original, correct } => {
                miscommand_text("Invalid", original, correct)
            }
            Suggestion::ContextHint { message, path } => hint_text(message, path.as_deref()),
            Suggestion::Fix { description, .. } => description.clone(),
        }
    }

    pub fn footer(&self) -> Option<String> {
        match self {
            Suggestion::DidYouMean { suggestion, .. } => Some(format!("→ Run: {}", suggestion)),
            Suggestion::Fix { command, .. } => {
                command.as_ref().map(|cmd| format!("→ Run: {}", cmd))
            }
            Suggestion::WrongOrder { correct, .. } => Some(format!("→ Run: {}", correct)),
            Suggestion::ContextHint { .. } => None,
        }
    }
}

/// Main suggestion engine for detecting and offering fixes.
pub struct SuggestionEngine {
    /// Similarity threshold for typo detection (0.0 to 1.0).
    similarity_threshold: f64,
}

impl Default for SuggestionEngine {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.6,
        }
    }
}

impl SuggestionEngine {
    /// Create a new suggestion engine with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new suggestion engine with custom similarity threshold.
    pub fn with_threshold(threshold: f64) -> Self {
        Self {
            similarity_threshold: threshold.clamp(0.0, 1.0),
        }
    }

    /// Find typo suggestions for an unknown command.
    ///
    /// Uses the `CommandRegistry` to find close matches.
    pub fn suggest_typo(&self, unknown: &str, registry: &CommandRegistry) -> Option<Suggestion> {
        let candidates = registry
            .all()
            .into_iter()
            .map(|cmd| (cmd, self.calculate_similarity(unknown, cmd)))
            .filter(|(_, similarity)| *similarity >= self.similarity_threshold);
        best_candidate(candidates).map(|(matched_cmd, _)| Suggestion::DidYouMean {
            original: unknown.to_string(),
            suggestion: matched_cmd.to_string(),
        })
    }

    /// Detect if commands are in wrong order.
    pub fn suggest_order(
        &self,
        first: &str,
        second: &str,
        valid_patterns: &[(&str, &str)],
    ) -> Option<Suggestion> {
        for (verb, noun) in valid_patterns {
            if *noun == first && *verb == second {
                return Some(Suggestion::WrongOrder {
                    original: format!("{} {}", first, second),
                    correct: format!("{} {}", second, first),
                });
            }
        }
        None
    }

    /// Calculate string similarity using Jaro-Winkler distance.
    fn calculate_similarity(&self, a: &str, b: &str) -> f64 {
        strsim::jaro_winkler(a, b)
    }
}

#[cfg(test)]
#[path = "suggestions/tests.rs"]
mod tests;

/// Compose the human message for a `Fix` suggestion.
fn fix_message(description: &str, command: Option<&str>) -> String {
    match command {
        Some(cmd) => format!("{}\n  → Run: {}", description, cmd),
        None => description.to_string(),
    }
}

/// The "…command '<original>'. Did you mean '<suggestion>'?" sentence shared
/// by `Suggestion::message` and `Suggestion::guidance`.
fn miscommand_text(kind: &str, original: &str, suggestion: &str) -> String {
    format!("{kind} command '{original}'. Did you mean '{suggestion}'?")
}

/// `"<message>: <path>"` when a path is attached, else the bare message.
fn hint_text(message: &str, path: Option<&str>) -> String {
    match path {
        Some(p) => format!("{message}: {p}"),
        None => message.to_string(),
    }
}

/// Parse a conventional fix hint into a `Suggestion::Fix`.
///
/// genesis-r13: hints are conventionally authored 'run: <cmd>' but the footer
/// already renders '→ Run: {cmd}' — keeping the prefix doubles it
/// ('→ Run: run: …'). Strip a leading 'run: '/'Run: ' at construction so the
/// stored command is runnable bare.
///
/// genesis-9us: trailing-command hints (specodelic's convention: 'every
/// reference resolves — run: specodelic lint') — the tail after the LAST
/// ' run: ' is the runnable command, the prefix is the guidance. Commands
/// are single lines; a multi-line tail is prose, not a command.
fn parse_fix_hint(s: &str) -> Option<Suggestion> {
    if let Some(stripped) = s.strip_prefix("run: ").or_else(|| s.strip_prefix("Run: ")) {
        let stripped = stripped.to_string();
        return Some(Suggestion::Fix {
            command: Some(stripped.clone()),
            description: if stripped == s {
                stripped
            } else {
                s.to_string()
            },
        });
    }
    trailing_command_hint(s)
}

/// Extract a trailing `' run: <cmd>'` hint when the tail is a plausible
/// single-line command.
fn trailing_command_hint(s: &str) -> Option<Suggestion> {
    let pos = s.rfind(" run: ")?;
    let cmd = &s[pos + " run: ".len()..];
    if cmd.contains('\n') || cmd.is_empty() {
        return None;
    }
    let description = s[..pos]
        .trim_end_matches([' ', ',', ';', '—', '-'])
        .to_string();
    Some(Suggestion::Fix {
        command: Some(cmd.to_string()),
        description,
    })
}

/// The highest-similarity candidate, first wins on ties.
fn best_candidate<'a>(candidates: impl Iterator<Item = (&'a str, f64)>) -> Option<(&'a str, f64)> {
    candidates.max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
}
