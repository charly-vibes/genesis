//! The canned, reusable check functions over a replayed [`ScenarioResult`].

use serde_json::Value;

use crate::evals::{
    AgentStep, CheckOutcome, Distractor, DistractorKind, EnvelopeOutcome, ErrorTaxonomy,
    EvalsError, ScenarioResult, parse_envelope,
};

// ---------------------------------------------------------------------------
// Reusable checks (core-slice helpers)
// ---------------------------------------------------------------------------

/// The replayed step at `step`, or a tool fault naming the gap.
fn replay_step(result: &ScenarioResult, step: usize) -> Result<&AgentStep, CheckOutcome> {
    result
        .steps
        .get(step)
        .ok_or_else(|| CheckOutcome::tool_fault(format!("no step {step} in replay")))
}

/// Assert `steps[i]` exited nonzero with a parseable error envelope
/// carrying `suggested_command` among its hints — the precondition for a
/// hint-following trial. Tool fault if the envelope is missing or hintless.
pub fn error_envelope_with_hint(
    step: usize,
    suggested_command: &str,
) -> impl Fn(&ScenarioResult) -> CheckOutcome + '_ {
    let suggested = suggested_command.to_owned();
    move |result| match replay_step(result, step) {
        Ok(s) => error_envelope_check(step, &suggested, s),
        Err(fault) => fault,
    }
}

/// Step-side classification for [`error_envelope_with_hint`]: the step
/// itself must have failed; envelope shape is [`hint_envelope_fault`].
fn error_envelope_check(step: usize, suggested: &str, s: &AgentStep) -> CheckOutcome {
    if s.exit_code == 0 {
        return CheckOutcome::tool_fault(format!(
            "step {step} exited 0; expected a failing invocation"
        ));
    }
    hint_envelope_fault(step, suggested, parse_envelope(&s.stdout))
}

/// Envelope-side classification for [`error_envelope_with_hint`]: pass
/// only when an error envelope carries `suggested` among its hints.
fn hint_envelope_fault(
    step: usize,
    suggested: &str,
    envelope: Result<EnvelopeOutcome, EvalsError>,
) -> CheckOutcome {
    match envelope {
        Ok(EnvelopeOutcome::Error { hint_commands, .. })
            if hint_commands.iter().any(|c| c == suggested) =>
        {
            CheckOutcome::pass()
        }
        Ok(EnvelopeOutcome::Error { hint_commands, .. }) => CheckOutcome::tool_fault(format!(
            "step {step} error envelope lacks hint `{suggested}` (hints: {hint_commands:?})"
        )),
        Ok(EnvelopeOutcome::Ok) => CheckOutcome::tool_fault(format!("step {step} emitted ok:true")),
        Err(e) => CheckOutcome::tool_fault(format!("step {step}: {e}")),
    }
}

/// Assert `steps[i]` exited 0 with an `ok:true` envelope — the graceful
/// recovery. Tool fault if the recovery envelope still fails: the agent
/// may have followed the hint and the fix itself was ineffective, so this
/// is *not* an agent-fault signal. Hint-blindness is classified by
/// [`agent_followed_hint`].
pub fn ok_envelope(step: usize) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    move |result| match replay_step(result, step) {
        Ok(s) => ok_envelope_check(step, s),
        Err(fault) => fault,
    }
}

/// Envelope-side classification for [`ok_envelope`]: exit 0 must pair
/// with an `ok:true` envelope; anything else is a tool fault.
fn ok_envelope_check(step: usize, s: &AgentStep) -> CheckOutcome {
    match parse_envelope(&s.stdout) {
        Ok(EnvelopeOutcome::Ok) if s.exit_code == 0 => CheckOutcome::pass(),
        Ok(EnvelopeOutcome::Ok) => CheckOutcome::tool_fault(format!(
            "step {step} emitted ok:true but exited {}",
            s.exit_code
        )),
        Ok(EnvelopeOutcome::Error { code, .. }) => CheckOutcome::tool_fault(format!(
            "step {step} still failed (code {:?}); the suggested fix was issued but ineffective",
            code
        )),
        Err(e) => CheckOutcome::tool_fault(format!("step {step}: {e}")),
    }
}

/// Assert the step's final envelope records a receipt whose
/// `terminal_outcome` is consistent with the envelope (add-artifact-provenance §4).
///
/// Passes when the envelope carries `receipt.terminal_outcome` and that
/// outcome does not contradict the envelope's `ok` field. Tool fault when
/// the receipt is missing entirely (the tool owns receipt emission on
/// mutating steps) or when a `success` receipt rides an `ok: false`
/// envelope — the lie the envelope exists to catch. The check itself does
/// not know which runs are mutating: the caller scopes it to steps it has
/// marked as mutating, mirroring the envelope contract.
pub fn receipt_records_terminal_outcome(step: usize) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    move |result| match replay_step(result, step) {
        Ok(s) => receipt_envelope_check(step, &s.stdout),
        Err(fault) => fault,
    }
}

/// Envelope-side classification for [`receipt_records_terminal_outcome`]:
/// the stdout JSON must carry `receipt.terminal_outcome` consistent with
/// the envelope's `ok` field.
fn receipt_envelope_check(step: usize, stdout: &str) -> CheckOutcome {
    let value: Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(e) => return CheckOutcome::tool_fault(format!("step {step}: envelope not JSON: {e}")),
    };
    match value
        .pointer("/receipt/terminal_outcome")
        .and_then(Value::as_str)
    {
        None => CheckOutcome::tool_fault(format!(
            "step {step}: mutating step ran without a receipt — envelope carries no receipt.terminal_outcome"
        )),
        Some("success") if value.get("ok").and_then(Value::as_bool) == Some(false) => {
            CheckOutcome::tool_fault(format!(
                "step {step}: receipt records terminal_outcome=success but the envelope has ok:false — contradiction"
            ))
        }
        Some(_) => CheckOutcome::pass(),
    }
}

/// Assert the agent's step at `recovery_index` actually ran the suggested
/// fix command. Agent fault (`ERR_ENVELOPE_HINT_BLINDNESS`) if a
/// different command was issued. Exact match counts, and so does the
/// suggested command as the word prefix: a recorded agent that followed
/// the hint but appended flags/args still followed it (mirrors
/// [`doc_drift_blindness`] matching; evallerina-2rr).
pub fn agent_followed_hint(
    recovery_index: usize,
    suggested_command: impl Into<String>,
) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let suggested = suggested_command.into();
    move |result| hint_follow_fault(result, recovery_index, &suggested)
}

/// Command-side classification for [`agent_followed_hint`]: pass when the
/// recorded command is the suggestion (modulo trailing flags/args).
fn hint_follow_fault(
    result: &ScenarioResult,
    recovery_index: usize,
    suggested: &str,
) -> CheckOutcome {
    let suggested_prefix = format!("{suggested} ");
    let Some(s) = result.steps.get(recovery_index) else {
        return hint_blindness_fault(format!("agent never ran the suggested `{suggested}`"));
    };
    if s.command == suggested || s.command.starts_with(&suggested_prefix) {
        return CheckOutcome::pass();
    }
    hint_blindness_fault(format!(
        "agent ran `{}` instead of the suggested `{suggested}`",
        s.command
    ))
}

/// An `EnvelopeHintBlindness` agent fault with a rendered reason.
fn hint_blindness_fault(reason: String) -> CheckOutcome {
    CheckOutcome::agent_fault(ErrorTaxonomy::EnvelopeHintBlindness, reason)
}

/// Assert every replayed step corresponds to an observed subprocess call.
/// Agent fault (`ERR_TOOL_EXECUTION_HALLUCINATION`) otherwise.
pub fn agent_executed_all() -> impl Fn(&ScenarioResult) -> CheckOutcome {
    |result| match result.steps.iter().position(|s| !s.executed) {
        None => CheckOutcome::pass(),
        Some(i) => CheckOutcome::agent_fault(
            ErrorTaxonomy::ToolExecutionHallucination,
            format!(
                "step {i} (`{}`) was never executed",
                result.steps[i].command
            ),
        ),
    }
}

/// Assert the agent trusted the tool's structured output over stale
/// documentation contradicting it (add-aix-eval-loop §3).
///
/// Requires a `StaleDocs` distractor declared via
/// [`Scenario::distractor_file`] — otherwise a tool fault (the check is
/// misconfigured, not the agent).
///
/// Passes when the replay shows the agent receiving a parseable envelope
/// from an executed step *and* never issuing the bait command (the action
/// that would only be correct per the stale docs). Action-level correctness
/// composes with [`ok_envelope`] / [`agent_followed_hint`]; this check owns
/// the drift signal alone. Failing steps classify as agent fault
/// (`ERR_DOC_DRIFT_BLINDNESS`) with the distractor path in the reason.
pub fn doc_drift_blindness(bait: impl Into<String>) -> impl Fn(&ScenarioResult) -> CheckOutcome {
    let bait = bait.into();
    move |result| {
        let Some(distractor) = stale_docs_distractor(result) else {
            return CheckOutcome::tool_fault(
                "doc_drift_blindness requires a StaleDocs distractor declared via Scenario::distractor_file",
            );
        };
        if let Some(fault) = bait_step_fault(result, &bait, distractor) {
            return fault;
        }
        executed_envelope_step(result).unwrap_or_else(|| {
            CheckOutcome::tool_fault(
                "no executed step received a parseable envelope; nothing to evaluate",
            )
        })
    }
}

/// The declared StaleDocs distractor, if any.
fn stale_docs_distractor(result: &ScenarioResult) -> Option<&Distractor> {
    result
        .distractors
        .iter()
        .find(|d| d.kind == DistractorKind::StaleDocs)
}

/// Agent fault when a step followed the stale-docs bait: exact command
/// match, or bait as the command word prefix (flags/args must not let
/// doc-following escape the check).
fn bait_step_fault(
    result: &ScenarioResult,
    bait: &str,
    distractor: &Distractor,
) -> Option<CheckOutcome> {
    let bait_prefix = format!("{bait} ");
    let step_index = result
        .steps
        .iter()
        .position(|s| s.command == bait || s.command.starts_with(&bait_prefix))?;
    Some(CheckOutcome::agent_fault(
        ErrorTaxonomy::DocDriftBlindness,
        format!(
            "step {step_index} ran `{bait}`, consistent with the stale docs at `{}` instead of the tool's envelope",
            distractor.path
        ),
    ))
}

/// The first executed step that received a parseable envelope, if any.
fn executed_envelope_step(result: &ScenarioResult) -> Option<CheckOutcome> {
    result
        .steps
        .iter()
        .position(|s| s.executed && parse_envelope(&s.stdout).is_ok())
        .map(|_| CheckOutcome::pass())
}
