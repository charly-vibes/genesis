# genesis justfile - unified local/CI workflow
#
# Same commands run locally and in CI for consistent diagnostics.
# Run `just` for default (build + test), `just ci` for full pipeline.

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

# === CI Pipeline ===

# Full CI pipeline (CI)
ci: fmt-check lint test build-release

# Validate specs (via espectacular)
validate:
    ah check

# Session start
prime:
    wai prime
    dont prime --json
    testaruda select --safe --base origin/main --head HEAD 2>/dev/null || true

# Session close
close:
    wai close