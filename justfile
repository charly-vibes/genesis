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

# Compile doc snippets + drift guards (doc_examples.rs mirrors, doc_sync.rs guards)
doc-test:
    cargo test --test doc_examples --test doc_sync

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

# Announce a release to downstream tools: opens an issue per repo via gh.
# Usage: just notify-downstream 0.7.0  (after tagging + crates.io publish)
notify-downstream version:
    #!/usr/bin/env bash
    set -euo pipefail
    repos=(wai dont espectacular testaruda vampiro dulce-de-leche pretender)
    mm="{{version}}"; mm="${mm%.*}"
    for repo in "${repos[@]}"; do
        if gh issue create -R "charly-vibes/$repo" \
            --title "genesis-vibes {{version}} released" \
            --body "genesis-vibes {{version}} is on crates.io. Changelog: https://github.com/charly-vibes/genesis/blob/main/CHANGELOG.md — note the manifest pin must be bumped (\"0.6\" → \"$mm\") since caret semantics exclude the new minor. Enable Dependabot (see .github/dependabot.yml in genesis) or bump manually." \
            >/dev/null 2>&1; then
            echo "✓ $repo notified"
        else
            echo "⚠ $repo skipped (gh failed — check auth/repo)"
        fi
    done

# Seed .github/dependabot.yml into downstream repos so Dependabot opens
# version-bump PRs when genesis-vibes publishes (run once, then commit+push
# in each repo). Idempotent: existing configs are left untouched.
seed-dependabot:
    #!/usr/bin/env bash
    set -euo pipefail
    repos=(wai dont espectacular testaruda vampiro dulce-de-leche pretender)
    root="$(git rev-parse --show-toplevel)"
    for repo in "${repos[@]}"; do
        d="$root/../$repo"
        cfg="$d/.github/dependabot.yml"
        if [[ -f "$cfg" ]]; then echo "= $repo (already configured)"; continue; fi
        mkdir -p "$d/.github"
        printf 'version: 2\nupdates:\n  - package-ecosystem: cargo\n    directory: /\n    schedule:\n      interval: weekly\n  - package-ecosystem: github-actions\n    directory: /\n    schedule:\n      interval: weekly\n' > "$cfg"
        echo "✓ $repo seeded → commit & push in $d"
    done

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