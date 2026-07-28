## 1. Verbosity
- [ ] 1.1 Define `Verbosity` enum (Quiet, Normal, Verbose, Debug).
- [ ] 1.2 Implement `From<u8>` for `Verbosity` (clap `-v` count).
- [ ] 1.3 `Verbosity::should_show(level)` — filter by minimum verbosity.
- [ ] 1.4 Unit tests for each tier.

## 2. Output type
- [ ] 2.1 Define `Output<T>` struct (data, next_step, warnings, verbosity).
- [ ] 2.2 `Output::print(&self, verbosity, &mut stdout, &mut stderr)` — formatted output.
- [ ] 2.3 `Output::to_envelope(&self) -> Envelope<T>` — JSON envelope output (for `--json`).
- [ ] 2.4 `Output::success(data) -> Self` and `Output::failure(err) -> Self` constructors.
- [ ] 2.5 `Output::with_next_step(self, hint: &str) -> Self` — fluent setter.
- [ ] 2.6 Unit tests for each display mode.

## 3. ErrorSink
- [ ] 3.1 Define `ErrorSink` struct (scratch, suggest, context, feedback_subcommand flags).
- [ ] 3.2 `ErrorSink::handle(&self, err, tool_name)` — prints error + suggestion footer.
- [ ] 3.3 `ErrorSink::handle_with_footer(&self, err, suggestion)` — explicit footer override.
- [ ] 3.4 Wire error scratch write (from `genesis::feedback::scratch`).
- [ ] 3.5 Wire feedback fallback when no fix exists (only if `feedback_subcommand` is set).
- [ ] 3.6 Unit tests for each error handling path.

## 4. Guide builder and runner
- [ ] 4.1 Define `GuideBuilder` struct with builder methods.
- [ ] 4.2 `Guide::new(name, version)` — creates a builder.
- [ ] 4.3 `.commands(list)` — registers commands with `CommandRegistry`.
- [ ] 4.4 `.config::<T>()` — enables ConfigStore (no-op if `genesis::config` not available).
- [ ] 4.5 `.verbosity(max)` — sets max verbosity level.
- [ ] 4.6 `.build()` — assembles all components into a `Guide`.
- [ ] 4.7 `Guide::run<F, T>(f)` — wraps execution: calls f, prints Output, handles errors.
- [ ] 4.8 `Guide::success(&self, output)` — prints guided success (used inside run).
- [ ] 4.9 `Guide::failure(&self, err)` — prints guided error via ErrorSink (used inside run).
- [ ] 4.10 Integration tests with a mock CLI tool.

## 5. Downstream migration
- [ ] 5.1 File adopt-guide issues for each tool.
- [ ] 5.2 Each tool replaces its main.rs setup with `Guide::new(...)`.
- [ ] 5.3 Each tool converts command handlers to return `Output<T>`.
- [ ] 5.4 Remove dead error-handling code from each tool.