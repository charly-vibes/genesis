# genesis-vibes justfile - unified local/CI workflow
#
# Same commands run locally and in CI for consistent diagnostics.
# Run `just` for default (build + test), `just ci` for full pipeline.
# Run `just publish` to publish a new version to crates.io.

set shell := ["bash", "-uc"]

# Default: build and test
default: build test

# === Build Commands ===

# Build debug binary
build:
    cargo build

# Build release binary (optimized)
build-release:
    cargo build --release

# Install locally to ~/.cargo/bin
install:
    cargo install --path .

# === Test Commands ===

# Run all tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run a specific test (e.g., `just test-name envelope_test`)
test-name name:
    cargo test {{name}} -- --nocapture

# Compile doc snippets against the real API (tests/doc_examples.rs mirrors mdBook snippets)
doc-test:
    cargo test --test doc_examples

# === Lint Commands ===

# Format code
fmt:
    cargo fmt

# Check formatting (CI)
fmt-check:
    cargo fmt --check

# Run clippy
lint:
    cargo clippy -- -D warnings

# === Docs Commands ===

# Build the mdBook documentation locally (requires mdbook)
docs:
    mdbook build

# === CI Pipeline ===

# Full CI pipeline — mirrors .github/workflows/ci.yml exactly
ci: fmt-check lint test docs build-release aix-check

# Validate specs (via espectacular)
validate:
    ah check

# Publish to crates.io (run after `just ci` passes)
publish:
    cargo publish

# Regenerate llms.txt and llm.txt from aix module metadata
aix-gen:
    cargo run --example gen-aix

# Verify AIX artifacts are up to date (CI use: "just aix-check" fails if stale)
aix-check: aix-gen
    git diff --exit-code llms.txt llm.txt

# Session start
prime:
    wai prime
    dont prime --json
    testaruda select --safe --base origin/main --head HEAD 2>/dev/null || true

# Session close
close:
    wai close

# Live preview docs at localhost:3000 (requires mdbook)
docs-serve:
    mdbook serve