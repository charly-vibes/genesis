# Design Decisions

**TL;DR:** Key trade-offs and rationale behind genesis-vibes's architecture — why the envelope is the single output format, why TTY detection matters, and how modules stay focused.

## Single envelope format

**Decision:** Every command returns an `Envelope<T>`. Callers check `ok` first.

**Rationale:** A single output format eliminates the "parse the help text" anti-pattern. Whether the consumer is a human, an agent, or a CI pipeline, the response shape is identical. The `ok` boolean is a universal success indicator; structured data is in `data`, errors in `error`.

**Trade-off:** Every command must construct an envelope. The overhead is negligible (a few allocations per command), but it imposes a discipline on output that some tools may find constraining.

## Mandatory error remediation

**Decision:** `ErrorResult::new()` returns `Err` if the remediation string is empty.

**Rationale:** An error without a suggested fix is a dead end for the user — and worse, a dead end for an AI agent that cannot ask for clarification. The `ErrorResult` constructor enforces that every error includes a recovery path.

**Trade-off:** Some errors genuinely have no remediation (e.g., disk full). In those cases, the remediation should say something like "Free up disk space and retry" — a general suggestion is better than none.

## TTY-aware output format

**Decision:** `CliFormat::format()` auto-detects stdout: TTY → `Human`, piped → `Json`.

**Rationale:** Agents and CI pipelines pipe stdout — they always get parseable JSON without any flags. Humans at a terminal get readable output. Either can be overridden with `--json` or `--human`.

**Trade-off:** Auto-detection can surprise users who pipe to `less` (TTY) and expect JSON. The explicit override exists for this case.

## Closed EnvelopeKind enum

**Decision:** `EnvelopeKind` is a closed enum — adding a variant requires a deliberate decision and a conformance test update.

**Rationale:** An open set of kinds would let each tool invent its own variant, fragmenting the protocol. A closed enum forces coordination: if you need a new kind, you modify genesis, and all tools benefit.

**Trade-off:** Tools that need a truly unique kind must either reuse an existing kind (e.g., `Info`) or open a discussion to extend the enum.

## DoctorCheck trait over function pointers

**Decision:** `DoctorCheck` is a trait with `name()`, `description()`, `run()`, `can_fix()`, `fix()`.

**Rationale:** A trait provides a stable contract that can be extended with new methods (e.g., `suggest()`) without breaking existing implementations. Function pointers would require a breaking change for any new capability.

**Trade-off:** More boilerplate for simple checks. A macro (`impl_doctor_check!`) could reduce this if needed.

## Discovery via TOML manifest

**Decision:** Tools register themselves in `.genesis/tools.toml` during `init`.

**Rationale:** Hardcoded tool lists in orchestrators like wai require a code change every time a tool is added or renamed. A filesystem manifest decouples tool registration from orchestrator code — wai just scans the manifest.

**Trade-off:** The manifest can drift from reality (a tool is removed but the manifest entry persists). The `DetectedTool::detected` field mitigates this by checking if the tool's marker still exists.

## Fixture uses real temp directories

**Decision:** `Fixture` creates real temporary directories on disk, not in-memory fakes.

**Rationale:** Integration tests that run real subprocesses need a real filesystem. In-memory fakes (e.g., `tempfile::TempDir`) would miss filesystem-level issues like permission errors, symlink resolution, and cross-device moves.

**Trade-off:** Slower than in-memory mocks. For unit tests that don't touch the filesystem, use standard Rust mocks.

## Scaffold returns created/existed paths

**Decision:** `Scaffold::build()` returns `ScaffoldResult` with `created` and `existed` path lists.

**Rationale:** Tools need to know which paths were newly created (for reporting) vs. which already existed (for idempotency). A simple boolean return would lose this information.

**Trade-off:** The caller must handle the result struct. For most use cases, the idiomatic pattern is `let result = scaffold.build().unwrap();` and ignore the result.