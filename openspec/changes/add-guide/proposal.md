# Change: Add `guide` module (scaffold for building guiding tools)

## Why

The tool-craft playbook defines five design principles for every tool in the
suite. Today each tool implements them independently — or not at all. The
Appendix A compliance matrix shows the gaps:

| Principle | wai | pretender | dont | espectacular | testaruda |
|---|---|---|---|---|---|
| Desire Path Alignment | ✓ | ✗ | ✗ | ✗ | ✗ |
| Self-Healing Errors | ✓ | ✗ | ✗ | ✗ | ✗ |
| Progressive Disclosure | ✓ | partial | partial | partial | ✗ |
| Context-Aware | ✓ | ✗ | ✗ | ✗ | ✗ |
| AIX | ✓ | ✓ | ✓ | ✓ | partial |

The existing genesis modules (suggestions, managed_block, aix, config) each
solve one piece, but there's no cohesive way to assemble them into a
*guiding tool*. Every new tool (crua, livin, vampiro) has to wire them
together from scratch.

A `genesis::guide` module provides a single entry point that assembles all
the pieces into a coherent CLI scaffold, so a new tool gets all five
principles for free.

## What Changes

### `Guide` struct — the single entry point

```rust
use genesis::guide::{Guide, Output};

let guide = Guide::new("my-tool", "0.1.0")
    .version("0.1.0")
    .about("Does one thing well")
    .verbosity(3)                          // -v, -vv, -vvv
    .commands(&["init", "check", "doctor"]) // for typo detection
    .config::<MyConfig>()?                 // if it has a config
    .build();
```

The `Guide` assembles and wires:
- `SuggestionEngine` (from `genesis::suggestions`) — typo detection, fix footers
- `CommandRegistry` (from `genesis::suggestions`) — command list registration
- `Verbosity` — three-tier progressive disclosure
- `ConfigStore` (from `genesis::config`) — config read/write/discover
- `BlockInjector` (from `genesis::managed_block`) — AGENTS.md managed blocks
- `ErrorSink` — error handler that always emits a `Suggestion::Fix` footer

### `Output` — every command produces a guided result

```rust
pub struct Output<T> {
    pub data: T,
    pub next_step: Option<Suggestion>,   // "→ Run: ..."
    pub warnings: Vec<Warning>,
    pub verbosity: u8,                    // what level to show
}
```

Every command handler returns `Output<T>`. The guide prints it:
- `data` → stdout (or JSON envelope if `--json`)
- `next_step` → footer stderr
- `warnings` → stderr
- `verbosity` → controls whether to show details

### `Verbosity` — progressive disclosure

```rust
pub enum Verbosity {
    Quiet = 0,    // -q: errors only
    Normal = 1,   // default: result + next step
    Verbose = 2,  // -v: +warnings + context
    Debug = 3,    // -vv: +internals + trace
}
```

### `ErrorSink` — self-healing errors

```rust
pub struct ErrorSink {
    /// The last error is persisted to the error scratch (§4 of feedback).
    pub scratch: bool,
    /// The error is printed with a Suggestion::Fix footer.
    pub suggest: bool,
    /// The error is printed with the full ContextBundle.
    pub context: bool,
}
```

Wired into the tool's `main.rs` error handler. Every non-zero exit:
1. Prints the error with a `Suggestion::Fix` footer
2. Writes to the error scratch (for `--from-last-error`)
3. If no fix exists, prints `Feedback: <tool> feedback bug --from-last-error`

### What a tool looks like with guide

```rust
fn main() -> Result<(), Box<dyn Error>> {
    let guide = Guide::new("my-tool", env!("CARGO_PKG_VERSION"))
        .commands(&["init", "check", "doctor"])
        .config::<MyConfig>()?
        .build();

    let result = match cli_command() {
        Ok(output) => guide.success(output),
        Err(err) => guide.failure(err),
    };
    // result always has a footer with next step or fix
    Ok(())
}
```

### What stays in each tool

- Domain logic (the actual work of `check`, `init`, `doctor`)
- Clap CLI definition (commands, args, flags)
- `Guide::new(...)` call (one-time setup, ~10 lines)

### What moves to genesis

- All five design principles from the playbook
- Verbosity tiers and their display logic
- Error → suggestion footer wiring
- Next-step hint after every command
- `Output` type for guided results
- `ErrorSink` for self-healing error handling
- Assembly of suggestions + managed_block + config + aix into one API

## Impact

- **New capability**: `guide` — CLI scaffold for building guiding tools.
- **Affected code**: new `src/guide/` module in genesis. Downstream tools
  replace their `main.rs` setup with `Guide::new(...)`.
- **Blocked by**: genesis v0.1.0 already tagged (guide can be v0.2.0).
- **Supersedes**: the ad-hoc pattern of each tool wiring suggestions,
  managed_block, config, and error handling independently.
- **Migration path**: additive — tools can adopt one feature at a time.
  `Guide::builder()` returns each component separately so a tool can adopt
  just `ErrorSink` without changing its command handlers.