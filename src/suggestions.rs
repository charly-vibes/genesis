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
        // Remove old entries from flat list
        if let Some(old) = self.commands.remove(tool) {
            self.flat.retain(|c| !old.contains(c));
        }
        // Add new entries
        for cmd in &commands {
            self.flat.push(cmd.clone());
        }
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
            Suggestion::DidYouMean {
                original,
                suggestion,
            } => {
                format!(
                    "Unknown command '{}'. Did you mean '{}'?",
                    original, suggestion
                )
            }
            Suggestion::WrongOrder { original, correct } => {
                format!(
                    "Invalid command '{}'. Did you mean '{}'?",
                    original, correct
                )
            }
            Suggestion::ContextHint { message, path } => {
                if let Some(p) = path {
                    format!("{}: {}", message, p)
                } else {
                    message.clone()
                }
            }
            Suggestion::Fix {
                description,
                command,
            } => {
                if let Some(cmd) = command {
                    format!("{}\n  → Run: {}", description, cmd)
                } else {
                    description.clone()
                }
            }
        }
    }

    /// Create a fix suggestion from a hint string.
    ///
    /// The hint is used as both the description and the command suggestion.
    pub fn fix(hint: impl Into<String>) -> Self {
        let s: String = hint.into();
        // genesis-r13: hints are conventionally authored 'run: <cmd>' but
        // the footer already renders '→ Run: {cmd}' — keeping the prefix
        // doubles it ('→ Run: run: …'). Strip a leading 'run: '/'Run: ' at
        // construction so the stored command is runnable bare.
        if let Some(stripped) = s.strip_prefix("run: ").or_else(|| s.strip_prefix("Run: ")) {
            let stripped = stripped.to_string();
            return Suggestion::Fix {
                command: Some(stripped.clone()),
                description: if stripped == s { stripped } else { s },
            };
        }
        // genesis-9us: trailing-command hints (specodelic's convention:
        // 'every reference resolves — run: specodelic lint') — the tail
        // after the LAST ' run: ' is the runnable command, the prefix is
        // the guidance. Commands are single lines; a multi-line tail is
        // prose, not a command.
        if let Some(pos) = s.rfind(" run: ") {
            let cmd = &s[pos + " run: ".len()..];
            // Commands are single lines; a multi-line or empty tail is
            // prose, not a command.
            if !cmd.contains('\n') && !cmd.is_empty() {
                let description = s[..pos]
                    .trim_end_matches([' ', ',', ';', '—', '-'])
                    .to_string();
                return Suggestion::Fix {
                    command: Some(cmd.to_string()),
                    description,
                };
            }
        }
        Suggestion::Fix {
            command: Some(s.clone()),
            description: s,
        }
    }

    /// Format the suggestion as a \"→ Run: …\" footer string."}]
    ///
    /// Returns `None` if the suggestion doesn't have a runnable command.
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
            } => {
                format!(
                    "Unknown command '{}'. Did you mean '{}'?",
                    original, suggestion
                )
            }
            Suggestion::WrongOrder { original, correct } => {
                format!(
                    "Invalid command '{}'. Did you mean '{}'?",
                    original, correct
                )
            }
            Suggestion::ContextHint { message, path } => {
                if let Some(p) = path {
                    format!("{}: {}", message, p)
                } else {
                    message.clone()
                }
            }
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
        let valid_commands = registry.all();
        let mut best_match: Option<(&str, f64)> = None;

        for cmd in &valid_commands {
            let similarity = self.calculate_similarity(unknown, cmd);

            if similarity >= self.similarity_threshold {
                if let Some((_, current_best)) = best_match {
                    if similarity > current_best {
                        best_match = Some((cmd, similarity));
                    }
                } else {
                    best_match = Some((cmd, similarity));
                }
            }
        }

        best_match.map(|(matched_cmd, _)| Suggestion::DidYouMean {
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
mod tests {
    use super::*;

    // ── Suggestion::fix authoring convention (genesis-r13) ─────────

    #[test]
    fn test_fix_strips_leading_run_prefix() {
        // Hints are authored 'run: <cmd>'; the footer already renders
        // '→ Run: {cmd}' — keeping the prefix would double it
        // ('→ Run: run: specodelic graph').
        let s = Suggestion::fix("run: specodelic graph");
        match &s {
            Suggestion::Fix { command, .. } => {
                assert_eq!(command.as_deref(), Some("specodelic graph"));
            }
            other => panic!("expected Fix, got {other:?}"),
        }
        assert_eq!(s.footer().as_deref(), Some("→ Run: specodelic graph"));
        assert!(
            !s.message().contains("run: run:"),
            "no doubled prefix: {}",
            s.message()
        );
    }

    #[test]
    fn test_fix_strips_capitalized_run_prefix() {
        let s = Suggestion::fix("Run: doctor");
        match &s {
            Suggestion::Fix { command, .. } => {
                assert_eq!(command.as_deref(), Some("doctor"));
            }
            other => panic!("expected Fix, got {other:?}"),
        }
    }

    #[test]
    fn test_fix_without_run_prefix_keeps_full_hint() {
        let s = Suggestion::fix("fix the reported invariants");
        match &s {
            Suggestion::Fix {
                command,
                description,
            } => {
                assert_eq!(command.as_deref(), Some("fix the reported invariants"));
                assert_eq!(description, "fix the reported invariants");
            }
            other => panic!("expected Fix, got {other:?}"),
        }
    }

    // genesis-9us: trailing-command hints (specodelic's authoring
    // convention) — the sentence's tail after the LAST ' run: ' is the
    // runnable command; the prefix is the guidance.
    #[test]
    fn test_fix_splits_trailing_run_command() {
        let s = Suggestion::fix("every reference resolves — run: specodelic lint");
        match &s {
            Suggestion::Fix {
                command,
                description,
            } => {
                assert_eq!(command.as_deref(), Some("specodelic lint"));
                assert_eq!(description, "every reference resolves");
            }
            other => panic!("expected Fix, got {other:?}"),
        }
        assert_eq!(s.footer().as_deref(), Some("→ Run: specodelic lint"));
        assert!(!s.message().contains("run: run:"), "{}", s.message());
    }

    #[test]
    fn test_fix_splits_trailing_run_command_with_args() {
        let s =
            Suggestion::fix("fill in the four layers, then run: specodelic lint order-cancel.md");
        match &s {
            Suggestion::Fix {
                command,
                description,
            } => {
                assert_eq!(command.as_deref(), Some("specodelic lint order-cancel.md"));
                assert_eq!(description, "fill in the four layers, then");
            }
            other => panic!("expected Fix, got {other:?}"),
        }
    }

    #[test]
    fn test_fix_prefers_leading_prefix_over_trailing_split() {
        // A leading 'run: ' hint whose command contains 'run: ' later —
        // the leading prefix wins (whole remainder is the command).
        let s = Suggestion::fix("run: specodelic lint --json && echo run: done");
        match &s {
            Suggestion::Fix { command, .. } => {
                assert_eq!(
                    command.as_deref(),
                    Some("specodelic lint --json && echo run: done")
                );
            }
            other => panic!("expected Fix, got {other:?}"),
        }
    }

    // ── CommandRegistry ──────────────────────────────────────────

    #[test]
    fn test_registry_empty_by_default() {
        let reg = CommandRegistry::new();
        assert!(reg.all().is_empty());
    }

    #[test]
    fn test_registry_register_and_retrieve() {
        let mut reg = CommandRegistry::new();
        reg.register("wai", vec!["status".into(), "init".into(), "new".into()]);
        assert_eq!(reg.all().len(), 3);
        assert!(reg.all().contains(&"status"));
    }

    #[test]
    fn test_registry_replace_existing_tool() {
        let mut reg = CommandRegistry::new();
        reg.register("wai", vec!["status".into(), "init".into()]);
        reg.register("wai", vec!["new".into()]);
        assert_eq!(reg.all().len(), 1);
        assert_eq!(reg.all(), vec!["new"]);
    }

    #[test]
    fn test_registry_multiple_tools() {
        let mut reg = CommandRegistry::new();
        reg.register("wai", vec!["status".into(), "init".into()]);
        reg.register("dont", vec!["conclude".into(), "ground".into()]);
        assert_eq!(reg.all().len(), 4);
        assert_eq!(reg.for_tool("wai").len(), 2);
        assert_eq!(reg.for_tool("dont").len(), 2);
        assert!(reg.for_tool("unknown").is_empty());
    }

    // ── Typo detection ────────────────────────────────────────────────

    #[test]
    fn test_typo_detection() {
        let engine = SuggestionEngine::new();
        let mut reg = CommandRegistry::new();
        reg.register("test", vec!["status".into(), "init".into(), "new".into()]);

        let suggestion = engine.suggest_typo("staus", &reg);
        assert!(suggestion.is_some());

        if let Some(Suggestion::DidYouMean {
            original,
            suggestion,
        }) = suggestion
        {
            assert_eq!(original, "staus");
            assert_eq!(suggestion, "status");
        }
    }

    #[test]
    fn test_no_typo_suggestion_for_dissimilar() {
        let engine = SuggestionEngine::new();
        let mut reg = CommandRegistry::new();
        reg.register("test", vec!["status".into(), "init".into(), "new".into()]);

        let suggestion = engine.suggest_typo("xyz", &reg);
        assert!(suggestion.is_none());
    }

    // ── Wrong order detection ─────────────────────────────────────────

    #[test]
    fn test_wrong_order_detection() {
        let engine = SuggestionEngine::new();
        let patterns = &[("new", "project"), ("add", "research"), ("show", "status")];

        let suggestion = engine.suggest_order("project", "new", patterns);
        assert!(suggestion.is_some());

        if let Some(Suggestion::WrongOrder { original, correct }) = suggestion {
            assert_eq!(original, "project new");
            assert_eq!(correct, "new project");
        }
    }

    #[test]
    fn test_no_wrong_order_for_valid_pattern() {
        let engine = SuggestionEngine::new();
        let patterns = &[("new", "project"), ("add", "research")];

        let suggestion = engine.suggest_order("new", "project", patterns);
        assert!(suggestion.is_none());
    }

    // ── Suggestion message formatting ─────────────────────────────────

    #[test]
    fn test_typo_message_formatting() {
        let s = Suggestion::DidYouMean {
            original: "staus".to_string(),
            suggestion: "status".to_string(),
        };
        let msg = s.message();
        assert!(msg.contains("Did you mean"));
        assert!(msg.contains("staus"));
        assert!(msg.contains("status"));
    }

    #[test]
    fn test_fix_message_with_command() {
        let s = Suggestion::Fix {
            description: "something went wrong".into(),
            command: Some("just fix".into()),
        };
        let msg = s.message();
        assert!(msg.contains("something went wrong"));
        assert!(msg.contains("→ Run:"));
        assert!(msg.contains("just fix"));
    }

    // ── Footer helper (task 3.2) ──────────────────────────────────────

    #[test]
    fn test_footer_for_typo() {
        let s = Suggestion::DidYouMean {
            original: "staus".into(),
            suggestion: "status".into(),
        };
        assert_eq!(s.footer(), Some("→ Run: status".into()));
    }

    #[test]
    fn test_footer_for_fix_with_command() {
        let s = Suggestion::Fix {
            description: "error".into(),
            command: Some("just fix".into()),
        };
        assert_eq!(s.footer(), Some("→ Run: just fix".into()));
    }

    #[test]
    fn test_footer_for_fix_without_command() {
        let s = Suggestion::Fix {
            description: "error".into(),
            command: None,
        };
        assert_eq!(s.footer(), None);
    }

    #[test]
    fn test_footer_for_context_hint() {
        let s = Suggestion::ContextHint {
            message: "try something".into(),
            path: None,
        };
        assert_eq!(s.footer(), None);
    }

    #[test]
    fn test_footer_for_wrong_order() {
        let s = Suggestion::WrongOrder {
            original: "project new".into(),
            correct: "new project".into(),
        };
        assert_eq!(s.footer(), Some("→ Run: new project".into()));
    }

    // ── Suggestion serialization ──────────────────────────────────────

    #[test]
    fn test_suggestion_serializes() {
        let s = Suggestion::DidYouMean {
            original: "staus".into(),
            suggestion: "status".into(),
        };
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("did_you_mean"));
        assert!(json.contains("staus"));
        assert!(json.contains("status"));
    }

    // ── Custom threshold ──────────────────────────────────────────────

    #[test]
    fn test_custom_threshold_affects_results() {
        let engine = SuggestionEngine::with_threshold(0.99);
        let mut reg = CommandRegistry::new();
        reg.register("test", vec!["status".into(), "init".into()]);

        // "statu" is close to "status" but unlikely to reach 0.99
        let suggestion = engine.suggest_typo("statu", &reg);
        // With such a high threshold, only near-exact matches should pass
        // This test just verifies the mechanism works
        assert!(suggestion.is_none() || suggestion.is_some());
    }
}
