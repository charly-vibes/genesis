# Agent Issue Reporting — `feedback` subcommand

> Status: **Draft** — a suite-wide subcommand that lets an agent (or human)
> file a well-contexted, well-tagged issue against a tool's upstream repo
> via the `gh` CLI, so friction observed in the field flows back to
> maintainers as structured signal rather than lost chat. Tracked by the
> `wai-bdqw` epic; implementation should be filed as an openspec change in
> wai.

This is the feature referenced by the tool-craft playbook §4 rule 5
("Errors link to the feedback loop").

---

## 0. Verb audit (why `feedback`, not `report`)

`report` is **already taken** in two suite tools with a different meaning:

- `pretender report` — render the last `check` run as human/markdown/HTML
  (`pretender/pretender/src/main.rs:60`, `ReportArgs`).
- `espectacular report` — render the coverage matrix
  (`espectacular/src/main.rs:49`, dispatches to `report::run_report`,
  `espectacular/src/report.rs`).

An earlier draft of this doc claimed "no charly-vibes tool ships a
feedback/issue command today" — that verification searched the wrong path
(`pretender/src` instead of the workspace member `pretender/pretender/src`)
and missed both. **`feedback` is free across all five CLIs** (verified
2026-07-27 against the real `src` of wai, pretender, dont, espectacular,
testaruda; see tool-craft.md Appendix A.2). It also matches the intent
precisely and avoids the render-vs-file ambiguity. The verb is `feedback`.

---

## 1. Why

Two gaps motivate this:

1. **Friction evaporates.** When an agent hits a self-healing failure (no
   `Suggestion::Fix` available) or a docs/AIX gap, the context — exact
   command, version, OS, what it tried — lives in that session and dies
   there. Maintainers never see it.
2. **Usage is invisible.** The suite is young; we don't know which verbs
   confuse agents, which errors have no fix, which docs are consulted most.
   A structured issue stream is the cheapest usage telemetry that respects
   user consent.

The loop this closes: **error with no fix → footer offers `feedback` →
maintainer receives a tagged issue → the fix becomes a `Suggestion::Fix` →
the footer stops offering `feedback` for that case.** Reporting should make
itself obsolete for any given error class.

---

## 2. Command grammar

Same verb on every charly-vibes tool, so an agent learns one command.

```
<tool> feedback [KIND] [--title <TITLE>] [--body <BODY>]
                 [--from-last-error] [--from-file <PATH>]
                 [--label <LABEL>...] [--no-label]
                 [--repo <OWNER/REPO>] [--dry-run] [--web] [--json]
                 [--yes] [--no-context] [--redact-remote]
```

### Kinds (closed set, maps to labels)

| Kind | When the agent uses it | Default labels |
|---|---|---|
| `bug` | Tool misbehaves: wrong output, panic, non-zero w/o fix. | `agent-reported`, `bug` |
| `friction` | Tool works but the path was rough (bad error, missing flag). | `agent-reported`, `friction` |
| `docs-gap` | Docs/llms.txt/AGENTS.md lacked what the agent needed. | `agent-reported`, `docs`, `aix` |
| `aix-gap` | Agent couldn't self-serve: no `--json`, no managed block, no prime. | `agent-reported`, `aix` |
| `idea` | Suggests a feature (not a defect). | `agent-reported`, `enhancement` |

`KIND` is optional; omitted ⇒ interactive prompt (cliclack, like `wai init`).

### Flags

| Flag | Purpose |
|---|---|
| `--from-last-error` | Attach the last error's `Suggestion` footer + exit code + command. Requires the tool to persist a one-line error scratch (see §4). |
| `--from-file <PATH>` | Attach a file's contents (e.g. a log, a config snippet). Redacted per §5. |
| `--label <LABEL>` | Add a label beyond the kind defaults. `--no-label` sends only these. |
| `--repo <OWNER/REPO>` | Override target repo (default = the tool's own `repository` field from `Cargo.toml`). Lets a tool file feedback against a *different* upstream (e.g. crua filing against a vampiro contract gap). |
| `--dry-run` | Print the title/body/labels and the exact `gh` invocation; do not create. |
| `--web` | Open the prefilled new-issue URL in a browser instead of `gh issue create` (fallback when `gh` absent/unauthed). |
| `--json` | On success, print `{"url": ..., "number": ...}` to stdout. |
| `--yes` | Skip the confirmation prompt (AFK). Default = show preview, require confirm (HITL). |
| `--no-context` | Omit the auto-gathered environment bundle (§3). For privacy-sensitive repos. |
| `--redact-remote` | Reduce `git_remote` to `host/path` (drop `user:pass@` and query). Default **on**; `--redact-remote=no` only for trusted private contexts. |

---

## 3. The context bundle

Auto-gathered, appended under a `## Context` heading so triage is fast and
reproducible. Everything here is **derived from the local machine**, not
from the user's source code (except an opt-in, redacted git summary).

```
- tool: <name> <version>          # from Cargo.toml + Cargo.lock pin
- command: <exact argv that ran> # the failing invocation, from the error scratch (§4)
- exit_code: <n>
- suggestion_footer: <the Suggestion::Fix message, if any>
- os/arch: <std::env::consts>
- shell: <SHELL>
- gh_version: <gh --version>     # or "(gh not found)"
- git_remote: <host/path>         # opt-in; reduced by --redact-remote (default on)
- git_branch: <branch>           # opt-in
- git_dirty: <bool>              # opt-in (no diff contents)
- repo_state: <which suite signals exist: .wai/ .beads/ openspec/ .dont/ .espectacular/>
- repro_hash: <sha256 of (tool+version+command+exit_code)>  # dedup key
```

`repo_state` is itself valuable telemetry: it tells maintainers *which
suite configurations are hitting which errors*.

**The whole body is redacted**, not just `--from-file` attachments — see §5.
`git_remote` never leaves the machine unreduced while `--redact-remote` is on
(default), because origin URLs routinely carry embedded credentials
(`https://<pat>@github.com/…`) or private hostnames.

---

## 4. The error scratch (prerequisite)

`--from-last-error` needs the last error available. Each tool persists, on
any non-zero exit, a one-line JSONL record to a cache dir:

```
$XDG_CACHE_HOME/<tool>/errors.jsonl
{"ts":"…","argv":["wai","status"],"exit":1,"footer":"→ Run: wai doctor","kind":"Fix"}
```

Rotated/capped (e.g. last 100). `feedback --from-last-error` reads the last
line. This is the same "persist the failure" idea wai already applies to
session state; it is not new infrastructure, just a new scratch file.

### Integration point

The write happens in **one place**: the top-level error sink in `main.rs`
(the miette report path), immediately before the process exits non-zero.
Gated by a config flag `error_scratch = true` (default on; airgapped users
turn it off).

### Hardening (must not shadow the real error)

- **Best-effort write.** Wrap in `let _ =` so a scratch failure never
  changes the exit code or masks the original error.
- **Read-only / no-cache-dir fallbacks.** If `$XDG_CACHE_HOME` is unset
  (Windows) or the dir is read-only (CI runners, Nix store), fall back to
  `std::env::temp_dir()`; if that also fails, silently skip — never panic.
- **Concurrency.** Open with append-mode + an advisory lock (or accept
  interleaved lines; JSONL is line-atomic). Two terminals reporting
  simultaneously must not corrupt the file.

🎯 **Target**: every tool writes this scratch on non-zero exit.

---

## 5. Privacy and consent

- **Never auto-send.** Default flow prints a full preview (title, body,
  labels, the `gh` command) and asks `Create this issue? [y/N]`. `--yes`
  for AFK must be an explicit, documented choice.
- **Redaction covers the whole body** — bundle (§3) *and* `--from-file`
  attachments. The redactor:
  - **Reduces `git_remote`** to `host/path` (default via `--redact-remote`);
    drops `user:pass@` and query string.
  - **Strips secret *values*** matching common patterns
    (`(?i)(token|secret|password|apikey|pat|bearer)`), env var *values*
    (`FOO=…` → `FOO=[redacted]`), and absolute home paths (→ `~/`).
  - **Matches values, not key substrings**, to avoid over-redacting legit
    terms like `monkey_type` or `keymap` — i.e. redact a value that *looks*
    like a secret, not any field whose *name* contains "key".
  - Marks elided ranges `[redacted]` so maintainers know something was cut.
- **No source contents.** The tool never attaches the user's source files.
  `--from-file` is opt-in per invocation.
- **`--no-context`** for users who want a bare report.
- **Repo targeting.** A report goes to the tool's *upstream* by default,
  never to the user's repo. The user's repo contributes only the opt-in,
  reduced `git_remote`/`branch`/`dirty` summary.

---

## 6. The issue body template

Stable, sectioned, so maintainers can triage by scanning headers.

```markdown
**Kind:** bug
**Repro hash:** <sha>

## Summary
<one line, from --title or the first line of --body>

## Reproduction
\`\`\`
$ <exact command>
\`\`\`
Exit code: <n>

## Expected
<what the agent expected>

## Actual / Suggestion footer
<the Suggestion::Fix wai offered, or "(no self-healing fix available)">

## Environment
<see Context bundle, redacted>

## Agent context
- working in: <repo_state signals>
- was following: <which managed block / ubiquitous-language entry, if known>
- trying to: <one line, from --body or interactive>

## Attached
<--from-file contents, redacted>
```

---

## 7. `gh` invocation and fallback

Primary path (requires `gh` on PATH and an authed default):

```bash
gh issue create \
  --repo <OWNER/REPO> \
  --title "<TITLE>" \
  --body-file - \
  --label "<l1>" --label "<l2>" ...
```

Body piped on stdin to avoid argv length limits and shell-escaping bugs.

**Fallback ladder** (self-healing, per playbook §4):

1. `gh` missing → print the body to a temp file and emit:
   `Suggestion::Fix { "gh not found; install GitHub CLI or open:", Some("open <prefilled-url>") }`
   where the URL is `https://github.com/<owner>/<repo>/issues/new?title=<urlenc>&body=<urlenc>`.
2. `gh` present but unauthed → same, with `gh auth login` in the fix footer.
3. Labels don't exist upstream → `gh issue create` without labels, and print a
   `Suggestion::ContextHint` "labels sync on next charly workflow run
   (§9)". The maintainer-side sync workflow (§9) closes it.
4. No network → write the body to `.<tool>/reports/<timestamp>.md` and tell
   the user where it is, with the `gh` command to retry later. `prime`
   (§10) reminds the user an unsent report exists.
5. **Permission error** (cross-repo case, §11) — `gh` returns
   `HTTP 403/404` or "resource not accessible": fall back to the `--web`
   prefilled URL and print a `Suggestion::ContextHint` "you may lack
   issue-create rights on the target; the web URL works for any logged-in
   user."

`--dry-run` short-circuits before any of this and prints the exact
invocation.

---

## 8. Label taxonomy (suite-wide, closed)

Labels must exist upstream or `gh` errors. Maintain a canonical set in the
charly monorepo and sync it (§9). The `feedback` command only ever uses these:

```
agent-reported            # always, on every agent-filed issue
bug | friction | docs | aix | enhancement   # the kind
no-self-healing-fix       # when --from-last-error and footer was empty
has-context-bundle        # the §3 bundle is attached
has-repro                 # repro_hash included
needs-triage              # default pending; removed on triage
```

Component labels (e.g. `commands/status`, `managed-block`, `aix`) are
tool-specific and declared per-repo in a `.github/labels.yml` synced from
charly.

---

## 9. Maintainer-side: label sync + triage automation

A charly-monorepo workflow (`/.github/workflows/sync-labels.yml`) pushes
the canonical label set to every tool repo on change, so `feedback` never
fails on a missing label (the §7 step-3 fallback's safety net). This is
the same "managed block" idea (playbook §5.1) applied to repo settings.

Optionally, a second workflow auto-closes `agent-reported` issues that lack
`has-repro` after 14 days, and auto-tags `no-self-healing-fix` issues into
the `Suggestion` backlog — closing the loop in §1.

---

## 10. AIX integration

- **Discoverable from errors.** When a tool exits non-zero with no
  `Suggestion::Fix`, the error footer ends with:
  `Feedback: <tool> feedback bug --from-last-error`
  This is the single most important integration — it is how the loop starts.
- **`prime` surfaces pending reports.** If `.<tool>/reports/` has unsent
  files (network was down), `prime` mentions "1 unsent report — run `<tool>
  feedback --send-pending`".
- **`--json` end-to-end.** An agent filing feedback gets back the issue URL
  and number, so it can record the reference in its handoff (`wai add
  research "Filed <tool>-123 for friction in status"`).
- **Documented in `llm.txt`.** Each tool's `llm.txt` gets a one-liner:
  `feedback  # file an issue against this tool's repo with context attached`.

---

## 11. Cross-repo reporting

`--repo <OWNER/REPO>` lets one tool file against another. Concrete use from
the suite evaluation: an agent discovering the crua↔livin hotness contract
gap (tickets `crua-mqt` / `livin-62a`) could run, from inside the crua repo:

```bash
crua feedback docs-gap --repo charly-vibes/livin \
  --title "EARS spec doesn't acknowledge crua hotness feed" \
  --body "crua-ears-spec.md §1 claims livin consumes hotness; livin-ears-spec.md §1 doesn't list it." \
  --from-file crua-ears-spec.md --yes
```

The `--from-file` attaches the relevant spec excerpt (redacted), and the
`repo_state` bundle shows the feedback came from a crua workspace.

**Cross-repo permission edge.** Filing against a repo the reporter isn't a
member of may require issues:create rights the reporter lacks; `gh` errors.
The §7 step-5 fallback handles this (→ `--web` URL + a hint). Any logged-in
user can open the prefilled URL even without create rights.

---

## 12. Scope and non-goals (v1)

**v1 does:**
- `bug`/`friction`/`docs-gap`/`aix-gap`/`idea` kinds.
- Auto context bundle (§3), error scratch (§4), full-body redaction (§5).
- `gh` primary + URL fallback ladder (§7), including permission edge.
- `--dry-run`, `--web`, `--json`, `--yes`, `--no-context`, `--redact-remote`.

**v1 does not:**
- Send anything automatically. There is no telemetry daemon, no background
  upload. The error scratch is local only.
- Attach source files or full logs. `--from-file` is the only attachment
  path and it is opt-in and redacted.
- File against a repo the tool isn't configured to target. The default is
  the tool's own `Cargo.toml` `repository` field; any other target must be
  passed explicitly.
- Judge feedback quality. Triage is human (or maintainer automation, §9).

---

## 13. Implementation plan (tracer-bullet)

Vertical slice that delivers value end-to-end before breadth:

1. **wai first** (reference impl). Add `feedback` to `commands/`, the error
   scratch in the `main.rs` error sink, the `Suggestion` footer hook in
   `suggestions.rs`. Wire `--from-last-error`. One e2e test runs
   `--dry-run` and asserts the body + the exact `gh` line + that a failed
   scratch write does not change the exit code. → demoable.
2. **Label set + sync workflow** in charly monorepo. → unblocks real filing.
3. **Port to dont/espectacular/testaruda/pretender** behind a **shared
   crate** — *not* a copy. The shared crate is `genesis` (per the
   `add-genesis-foundation` proposal); the redactor, context-bundle
   serializer, and `feedback` module are candidates for it. Crate home:
   `genesis` (the `feedback` module; wai is the reference impl and donor —
   see `add-genesis-foundation` task 5.x). The decision is made here, not
   deferred.
4. **Maintainer triage workflow** (§9) once feedback exists.

Each step is independently demoable — no step is "plumbing only."

---

## Traceability

- Implements playbook §4 rule 5 ("Errors link to the feedback loop").
- Verb audit (§0) closes tool-craft.md §2.5 (reserved verbs).
- Depends on: the error-scratch infra (§4) and the label-sync workflow (§9).
- Should be filed as an openspec change in wai (`openspec/changes/`) per
  wai's convention, and as `wai-bdqw`-style tickets in the other repos once
  the wai reference lands.
- Relates to `dont-2j6o` (shared JSON envelope crate) — the redactor,
  context-bundle serializer, and this feature's port are all candidates for
  that shared crate (§13 step 3).
