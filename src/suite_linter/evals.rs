//! Evals-guidelines adoption probe (`add-evals-guidelines`, task 5.3).
//!
//! - `EvalsGuidelinesAdoption` — advisory check that a consumer repo's
//!   eval docs reference the genesis guideline pages (the report contract
//!   and the tier-ladder how-tos)
//! - `collect_eval_docs` — recursive `evals*.md` discovery under `docs/`

use std::path::Path;

use super::{LintCheck, LintResult, Severity};

/// Adoption probe for the suite-wide evals guidelines
/// (`add-evals-guidelines`, task 5.3): verifies that a consumer repo's
/// eval docs reference the genesis guideline pages (the report contract
/// and the tier-ladder how-tos). Advisory — an adoption probe, not a gate.
/// Register it in the consumer's linter registry at startup.
pub struct EvalsGuidelinesAdoption;

/// Marker strings that count as "references the guidelines" — any one hit
/// per eval doc satisfies the probe.
const EVALS_GUIDELINE_MARKERS: &[&str] = &[
    "eval-report",
    "evals-guidelines",
    "evals-ci",
    "why-weak-readers",
];

/// The advisory result for a repo with no eval docs at all.
fn not_adopted() -> LintResult {
    LintResult::new(
        "no eval docs found — evals-guidelines not yet adopted (battery not started)",
        Severity::Advisory,
    )
}

impl LintCheck for EvalsGuidelinesAdoption {
    fn name(&self) -> &'static str {
        "genesis.evals_guidelines"
    }

    fn description(&self) -> &'static str {
        "eval docs reference the suite-wide evals-guidelines pages"
    }

    fn run(&self, repo_root: &Path) -> Result<Vec<LintResult>, Box<dyn std::error::Error>> {
        let docs = repo_root.join("docs");
        let eval_docs = if docs.is_dir() {
            collect_eval_docs(&docs)?
        } else {
            Vec::new()
        };
        if eval_docs.is_empty() {
            return Ok(vec![not_adopted()]);
        }
        let findings = eval_docs
            .iter()
            .filter_map(|doc| check_eval_doc(repo_root, doc))
            .collect();
        Ok(findings)
    }
}

/// Advisory finding when `doc` does not reference any guideline marker.
fn check_eval_doc(repo_root: &Path, doc: &std::path::Path) -> Option<LintResult> {
    let content = std::fs::read_to_string(doc)
        .map_err(|e| format!("cannot read {}: {e}", doc.display()))
        .ok()?;
    let references = EVALS_GUIDELINE_MARKERS.iter().any(|m| content.contains(m));
    if references {
        return None;
    }
    let rel = doc.strip_prefix(repo_root).unwrap_or(doc).display();
    Some(LintResult::with_fix(
        format!(
            "{rel} does not reference the suite-wide evals-guidelines pages \
             (report contract / tier ladder) — cross-tool results are not \
             comparable without the shared conventions"
        ),
        Severity::Advisory,
        "see genesis docs: how-to/evals.md, how-to/evals-ci.md, \
         reference/eval-report.md",
    ))
}

/// True when `path` names an `evals*.md` doc.
fn is_eval_doc(path: &std::path::Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("evals") && name.ends_with(".md"))
}

/// Queue subdirectories and record this directory's `evals*.md` docs.
fn scan_dir(
    eval_docs: &mut Vec<std::path::PathBuf>,
    stack: &mut Vec<std::path::PathBuf>,
    dir: std::path::PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.is_dir() {
            stack.push(path);
        } else if is_eval_doc(&path) {
            eval_docs.push(path);
        }
    }
    Ok(())
}

/// Collect `evals*.md` files under `docs/`, any depth.
fn collect_eval_docs(
    docs: &std::path::Path,
) -> Result<Vec<std::path::PathBuf>, Box<dyn std::error::Error>> {
    let mut eval_docs: Vec<std::path::PathBuf> = Vec::new();
    let mut stack = vec![docs.to_path_buf()];
    while let Some(dir) = stack.pop() {
        scan_dir(&mut eval_docs, &mut stack, dir)?;
    }
    Ok(eval_docs)
}
