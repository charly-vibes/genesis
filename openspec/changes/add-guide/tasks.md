## 1. Verbosity
- [x] 1.1 Define `Verbosity` enum (Quiet, Normal, Verbose, Debug).
- [x] 1.2 Implement `From<u8>` for `Verbosity` (clap `-v` count).
- [x] 1.3 `Verbosity::should_show(level)` — filter by minimum verbosity.
- [x] 1.4 Unit tests for each tier.

## 2. Output type
- [x] 2.1 Define `Output<T>` struct (data, next_step, warnings, verbosity).
- [x] 2.2 `Output::print(&self, verbosity, &mut stdout, &mut stderr)` — formatted output.
- [x] 2.3 `Output::to_envelope(&self) -> Envelope<T>` — JSON envelope output (for `--json`).
- [x] 2.4 `Output::success(data) -> Self` and `Output::failure(err) -> Self` constructors.
- [x] 2.5 `Output::with_next_step(self, hint: &str) -> Self` — fluent setter.
- [x] 2.6 Unit tests for each display mode.

## 3. ErrorSink
- [x] 3.1 Define `ErrorSink` struct (scratch, suggest, context, feedback_subcommand flags).
- [x] 3.2 `ErrorSink::handle(&self, err, tool_name)` — prints error + suggestion footer.
- [x] 3.3 `ErrorSink::handle_with_footer(&self, err, suggestion)` — explicit footer override.
- [x] 3.4 Wire error scratch write (from `genesis::feedback::scratch`).
- [x] 3.5 Wire feedback fallback when no fix exists (only if `feedback_subcommand` is set).
- [x] 3.6 Unit tests for each error handling path.

## 4. Guide builder and runner
- [x] 4.1 Define `GuideBuilder` struct with builder methods.
- [x] 4.2 `Guide::new(name, version)` — creates a builder.
- [x] 4.3 `.commands(list)` — registers commands with `CommandRegistry`.
- [x] 4.4 `.config::<T>()` — enables ConfigStore (no-op if `genesis::config` not available).
- [x] 4.5 `.verbosity(max)` — sets max verbosity level.
- [x] 4.6 `.build()` — assembles all components into a `Guide`.
- [x] 4.7 `Guide::run<F, T>(f)` — wraps execution: calls f, prints Output, handles errors.
- [x] 4.8 `Guide::success(&self, output)` — prints guided success (used inside run).
- [x] 4.9 `Guide::failure(&self, err)` — prints guided error via ErrorSink (used inside run).
- [x] 4.10 Integration tests with a mock CLI tool.

## 5. Downstream migration
- [ ] 5.1 File adopt-guide issues for each tool.
- [ ] 5.2 Each tool replaces its main.rs setup with `Guide::new(...)`.
- [ ] 5.3 Each tool converts command handlers to return `Output<T>`.
- [ ] 5.4 Remove dead error-handling code from each tool.