#!/usr/bin/env bash
# Purpose: fail when committed BRIEF.md / BRIEF.json no longer match a regeneration.
#
# Responsibilities:
# - no-op when the repository has no committed renders; refuse a half-present pair;
# - regenerate both renders into a temporary directory at the committed render's
#   evaluation date (or an explicit date);
# - normalize volatile and clone-local fields, then diff content;
# - exit 0 on a match, 1 on drift (naming the artifact), 2 on usage/dependency errors;
# - never write the working tree.
#
# Rationale: a committed render is only reviewable if drift between it and its
# inputs fails loudly. Regeneration targets a temp directory because `podras
# brief --write` defaults to the repository root; the committed render records
# its own evaluation date in `as_of`, so the gate compares the same clock the
# render was built with instead of drifting with today.
#
# Usage: scripts/guards/brief-drift.sh [YYYY-MM-DD]
#   The date argument overrides $PODRAS_DRIFT_AS_OF, which overrides the
#   recorded `.data.as_of` (falling back to the date prefix of
#   `.data.generated_at` for renders written before as_of was recorded).
#   $PODRAS_BIN selects the podras binary; the default is the repository's
#   target/debug/podras when executable, else `podras` from PATH.
#
# Opt-in wiring: see the commented `brief-drift` stanza in lefthook.yml.

set -euo pipefail

readonly MARKDOWN=BRIEF.md
readonly JSON=BRIEF.json

err() {
    printf 'brief-drift: %s\n' "$*" >&2
}

usage() {
    printf 'usage: %s [YYYY-MM-DD]\n' "${0##*/}" >&2
}

if [[ $# -gt 1 ]]; then
    usage
    exit 2
fi

root=$(git rev-parse --show-toplevel 2>/dev/null) || {
    err "not inside a git repository"
    exit 2
}
cd "$root"

if [[ ! -e $MARKDOWN && ! -e $JSON ]]; then
    printf 'brief-drift: no committed renders (%s, %s); nothing to check\n' "$MARKDOWN" "$JSON"
    exit 0
fi
if [[ ! -e $MARKDOWN || ! -e $JSON ]]; then
    err "incomplete committed renders: $MARKDOWN and $JSON must exist together"
    exit 1
fi

command -v jq >/dev/null 2>&1 || {
    err "jq is required to normalize committed renders"
    exit 2
}

podras_bin=${PODRAS_BIN:-}
if [[ -z $podras_bin ]]; then
    # Prefer the repository's own build under test, then the PATH binary.
    if [[ -x $root/target/debug/podras ]]; then
        podras_bin=$root/target/debug/podras
    else
        podras_bin=podras
    fi
fi
if [[ $podras_bin == */* ]]; then
    [[ -x $podras_bin ]] || {
        err "podras binary is not executable: $podras_bin"
        exit 2
    }
else
    command -v "$podras_bin" >/dev/null 2>&1 || {
        err "podras binary not found: $podras_bin (set PODRAS_BIN)"
        exit 2
    }
fi

as_of=${1:-${PODRAS_DRIFT_AS_OF:-}}
if [[ -z $as_of ]]; then
    as_of=$(jq -r '(.data.as_of // (.data.generated_at | .[0:10])) // empty' "$JSON")
    if [[ -z $as_of ]]; then
        err "cannot read .data.as_of from $JSON; pass YYYY-MM-DD or set PODRAS_DRIFT_AS_OF"
        exit 2
    fi
fi
if [[ ! $as_of =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
    err "invalid evaluation date '$as_of' (expected YYYY-MM-DD)"
    exit 2
fi

tmp=$(mktemp -d "${TMPDIR:-/tmp}/podras-brief-drift.XXXXXX")
trap 'rm -rf "$tmp"' EXIT

tree_before=$(git status --porcelain)

render_log=$("$podras_bin" brief --write --as-of "$as_of" --out-dir "$tmp" 2>&1) &&
    render_status=0 || render_status=$?
if ((render_status != 0)); then
    if ((render_status == 2)); then
        err "podras rejected the invocation or evaluation date '$as_of':"
        printf '%s\n' "$render_log" >&2
        exit 2
    fi
    err "podras brief --write --out-dir failed (exit $render_status):"
    printf '%s\n' "$render_log" >&2
    exit 1
fi
if [[ ! -e $tmp/$MARKDOWN || ! -e $tmp/$JSON ]]; then
    err "regeneration produced no render pair in $tmp"
    exit 1
fi

# JSON normalization: drop the wall-clock stamp, the content hash, per-source
# warnings, and clone-local provenance (the curation file mtime, per-source
# observed_at/staleness, and absolute-path details) from every row.
normalize_json() {
    jq 'del(.data.generated_at, .data.source_hash, .data.warnings, .meta.author)
        | del(.data.curation.updated_at)
        | (.data.sources[]? |= del(.observed_at, .staleness, .detail))' "$1"
}

# Markdown normalization: compare the narrative through the health section and
# stop at the final sources/freshness section, whose mtimes and stamp are
# clone-local. This detects content drift, not freshness drift.
normalize_markdown() {
    awk '
        /^## Sources & freshness/ { exit }
        /^Freshness: generated_at / { next }
        { print }
    ' "$1"
}

if ! normalize_json "$root/$JSON" >"$tmp/$JSON.committed.norm"; then
    err "cannot normalize committed $JSON"
    exit 1
fi
if ! normalize_json "$tmp/$JSON" >"$tmp/$JSON.regenerated.norm"; then
    err "cannot normalize regenerated $JSON"
    exit 1
fi
if ! normalize_markdown "$root/$MARKDOWN" >"$tmp/$MARKDOWN.committed.norm"; then
    err "cannot normalize committed $MARKDOWN"
    exit 1
fi
if ! normalize_markdown "$tmp/$MARKDOWN" >"$tmp/$MARKDOWN.regenerated.norm"; then
    err "cannot normalize regenerated $MARKDOWN"
    exit 1
fi

drift_count=0
for artifact in "$MARKDOWN" "$JSON"; do
    if ! diff -u \
        --label "$artifact (committed)" \
        --label "$artifact (regenerated)" \
        "$tmp/$artifact.committed.norm" \
        "$tmp/$artifact.regenerated.norm" >"$tmp/$artifact.diff"; then
        drift_count=$((drift_count + 1))
    fi
done

tree_after=$(git status --porcelain)
if [[ $tree_before != "$tree_after" ]]; then
    err "the gate changed the working tree; it must never write the repository"
    exit 1
fi

if ((drift_count > 0)); then
    for artifact in "$MARKDOWN" "$JSON"; do
        if [[ -s $tmp/$artifact.diff ]]; then
            err "$artifact drifted from a fresh render at $as_of"
            cat "$tmp/$artifact.diff" >&2
        fi
    done
    err "re-run: podras brief --write --as-of $as_of"
    exit 1
fi

printf 'brief-drift: %s and %s match a fresh render at %s\n' "$MARKDOWN" "$JSON" "$as_of"
exit 0